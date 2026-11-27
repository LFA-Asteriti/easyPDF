//! Native, loopback-only Sign in with ChatGPT for open-source applications.
//! The installed app communicates directly with OpenAI and uses the OS credential store.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use easypdf_ui_egui::connected::{AiTask, Command, Event, LocalChatGptCommand, LocalChatGptModel, LocalChatGptProfile};
use ring::{digest, rand, signature};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use url::Url;

use crate::portal::{configured_agent, credential_for};

const ISSUER: &str = "https://auth.openai.com";
const AUTHORIZE: &str = "https://auth.openai.com/api/accounts/authorize";
const TOKEN: &str = "https://auth.openai.com/api/accounts/oauth/token";
const JWKS: &str = "https://auth.openai.com/.well-known/jwks.json";
const DISCOVERY: &str = "https://auth.openai.com/.well-known/openid-configuration";
const RESOURCE: &str = "https://api.openai.com/v1";
const CALLBACK: &str = "/auth/callback";
const SCOPES: &str = "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct";
const PLAN_SCOPE: &str = "chatgpt.tokens.use.direct";
const PROFILE: &str = "chatgpt-local-profile";
const HOST: &str = "chatgpt-local-host";
const MAX_JSON: u64 = 2_000_000;

#[derive(Serialize, Deserialize)]
struct Profile {
    client_id: String,
    subject: String,
    account: String,
    refresh_token: String,
    model: String,
    active: bool,
}

#[derive(Deserialize)]
struct Tokens {
    access_token: String,
    refresh_token: Option<String>,
    id_token: Option<String>,
    #[serde(default)]
    expires_in: u64,
    scope: Option<String>,
}

#[derive(Deserialize)]
struct Identity {
    sub: String,
    email: Option<String>,
    iss: String,
    aud: Value,
    azp: Option<String>,
    exp: u64,
    iat: u64,
    nonce: String,
}

#[derive(Default)]
pub(crate) struct Session {
    profile: Option<Profile>,
    access: Option<(String, Instant)>,
    models: Vec<LocalChatGptModel>,
}

pub(crate) fn is_local(command: &Command) -> bool {
    matches!(command, Command::LocalChatGpt(_))
}

impl Session {
    pub(crate) fn restore(&mut self) -> Result<Option<LocalChatGptProfile>, String> {
        let stored: Option<Profile> = read_credential(PROFILE)?
            .map(|bytes| serde_json::from_slice(&bytes))
            .transpose()
            .map_err(|_| "Saved ChatGPT session is unreadable.".to_string())?;
        let same_credential = match (&stored, &self.profile) {
            (Some(current), Some(previous)) => current.client_id == previous.client_id && current.refresh_token == previous.refresh_token,
            (None, None) => true,
            _ => false,
        };
        if !same_credential {
            self.access = None;
            self.models.clear();
        }
        self.profile = stored;
        Ok(self.public_profile())
    }

    fn profile_event(&self) -> Event {
        Event::LocalChatGptProfile(self.public_profile())
    }

    fn public_profile(&self) -> Option<LocalChatGptProfile> {
        self.profile.as_ref().map(|profile| LocalChatGptProfile {
            account: profile.account.clone(),
            model: profile.model.clone(),
            active: profile.active,
            connected: !profile.refresh_token.is_empty(),
        })
    }

    pub(crate) fn execute(&mut self, command: Command) -> Result<Event, String> {
        let Command::LocalChatGpt(command) = command else { return Err("Invalid local ChatGPT command.".into()) };
        // A refresh token may rotate. Lock and reread the shared record even when
        // two desktop processes were launched with different command-line options.
        let _lock = lock_profile()?;
        self.restore()?;
        match command {
            LocalChatGptCommand::SignIn => self.sign_in(false),
            LocalChatGptCommand::SignInNew => self.sign_in(true),
            LocalChatGptCommand::SignOut => self.sign_out(),
            LocalChatGptCommand::SetActive(active) => {
                if let Some(profile) = &mut self.profile {
                    let previous = profile.active;
                    profile.active = active;
                    if let Err(error) = save_profile(profile) {
                        profile.active = previous;
                        return Err(error);
                    }
                }
                Ok(self.profile_event())
            }
            LocalChatGptCommand::LoadModels => {
                let agent = configured_agent(0)?;
                let access = self.access(&agent)?;
                self.models = load_models(&agent, access)?;
                Ok(Event::LocalChatGptModels(self.models.clone()))
            }
            LocalChatGptCommand::SaveModel(model) => {
                if !self.models.iter().any(|item| item.slug == model) {
                    return Err("Choose a model from the ChatGPT account’s model list.".into());
                }
                let profile = self.profile.as_mut().ok_or("Connect ChatGPT first.")?;
                let previous = std::mem::replace(&mut profile.model, model);
                if let Err(error) = save_profile(profile) {
                    profile.model = previous;
                    return Err(error);
                }
                Ok(self.profile_event())
            }
            LocalChatGptCommand::Run { task, text, question, language } => self.run(task, &text, &question, &language),
        }
    }

    fn sign_in(&mut self, new_account: bool) -> Result<Event, String> {
        if new_account && self.profile.as_ref().is_some_and(|profile| !profile.refresh_token.is_empty()) {
            return Err("Disconnect the current ChatGPT account before connecting a different one.".into());
        }
        let agent = configured_agent(0)?;
        let host = host_id()?;
        let request = SignIn::new()?;
        let previous = self.profile.as_ref().filter(|_| !new_account);
        let current_id = previous.map(|profile| profile.client_id.as_str());
        webbrowser::open(request.authorization_url(&host, current_id).as_str())
            .map_err(|_| "Could not open the browser for ChatGPT authorization.")?;
        let (code, client_id) = request.callback(current_id)?;
        let tokens = token_request(
            &agent,
            &[
                ("grant_type", "authorization_code"),
                ("client_id", &client_id),
                ("code", &code),
                ("code_verifier", &request.verifier),
                ("redirect_uri", &request.redirect),
                ("resource", RESOURCE),
            ],
        )?;
        let claims = verify_identity(&agent, tokens.id_token.as_deref().ok_or("OpenAI did not return an ID token.")?, &client_id, &request.nonce)?;
        if let Some(profile) = previous
            && claims.sub != profile.subject
        {
            return Err("A different ChatGPT account was authorized. Disconnect the saved account before switching.".into());
        }
        require_plan(tokens.scope.as_deref())?;
        let refresh = tokens.refresh_token.as_deref().filter(|token| !token.is_empty()).ok_or("OpenAI did not return a renewable session.")?;
        let profile = Profile {
            client_id,
            subject: claims.sub,
            account: claims.email.filter(|email| !email.is_empty()).unwrap_or_else(|| "ChatGPT".into()),
            refresh_token: refresh.into(),
            model: previous.map_or_else(String::new, |profile| profile.model.clone()),
            active: true,
        };
        save_profile(&profile)?;
        self.profile = Some(profile);
        self.access = Some(cache_access(tokens));
        self.models.clear();
        Ok(self.profile_event())
    }

    fn sign_out(&mut self) -> Result<Event, String> {
        let profile = self.profile.as_mut().ok_or("Connect ChatGPT first.")?;
        let refresh = std::mem::take(&mut profile.refresh_token);
        self.access = None;
        self.models.clear();
        // Keep the issued client registration for reauthorization, without
        // retaining the renewable session after sign-out.
        if let Err(error) = save_profile(profile) {
            profile.refresh_token = refresh;
            return Err(error);
        }
        let warning = (!refresh.is_empty()).then(|| revoke(&profile.client_id, &refresh)).and_then(Result::err);
        Ok(Event::LocalChatGptSignedOut { profile: self.public_profile(), warning })
    }

    fn access(&mut self, agent: &ureq::Agent) -> Result<&str, String> {
        let fresh = self.access.as_ref().is_some_and(|(_, until)| *until > Instant::now());
        if !fresh {
            let profile = self.profile.as_mut().filter(|profile| !profile.refresh_token.is_empty()).ok_or("Connect ChatGPT first.")?;
            let tokens = token_request(
                agent,
                &[
                    ("grant_type", "refresh_token"),
                    ("client_id", &profile.client_id),
                    ("refresh_token", &profile.refresh_token),
                    ("resource", RESOURCE),
                ],
            )
            .map_err(|error| format!("{error} Select Continue with ChatGPT to reconnect."))?;
            if let Some(scope) = tokens.scope.as_deref() {
                require_plan(Some(scope))?;
            }
            if let Some(refresh) = tokens.refresh_token.as_ref().filter(|token| !token.is_empty() && **token != profile.refresh_token) {
                profile.refresh_token.clone_from(refresh);
                save_profile(profile)?;
            }
            self.access = Some(cache_access(tokens));
        }
        self.access.as_ref().map(|(token, _)| token.as_str()).ok_or_else(|| "ChatGPT session expired.".to_string())
    }

    fn run(&mut self, task: AiTask, text: &str, question: &str, language: &str) -> Result<Event, String> {
        if text.trim().is_empty() || text.chars().count() > easypdf_ui_egui::connected::MAX_AI_TEXT_CHARS {
            return Err("Document text is empty or too large.".into());
        }
        let instructions = match task {
            AiTask::Summary => "Summarize the document concisely. Preserve key facts, dates, people, decisions and action items.".into(),
            AiTask::Ask if !question.trim().is_empty() && question.chars().count() <= 4000 => {
                format!(
                    "Use only the document to answer this question. Cite relevant [Page N] markers and say if the answer is absent. Question: {question}"
                )
            }
            AiTask::Translate if !language.trim().is_empty() && language.chars().count() <= 80 => {
                format!("Translate the document faithfully into {language}. Preserve headings and lists.")
            }
            _ => return Err("Enter a valid question or target language.".into()),
        };
        let model = self
            .profile
            .as_ref()
            .filter(|profile| profile.active && !profile.model.is_empty())
            .ok_or("Choose and save a ChatGPT model first.")?
            .model
            .clone();
        let agent = configured_agent(0)?;
        let access = self.access(&agent)?;
        let body = json!({
            "model": model,
            "instructions": instructions,
            "input": [{"role": "user", "content": text}],
            "store": false,
            "stream": true
        });
        let mut response = agent
            .post(format!("{RESOURCE}/responses"))
            .header("Authorization", format!("Bearer {access}"))
            .header("Content-Type", "application/json")
            .send(body.to_string())
            .map_err(|error| network_error("ChatGPT request failed", &error))?;
        let stream = response.body_mut().with_config().limit(8_000_000).reader();
        parse_stream(BufReader::new(stream)).map(Event::AiResult)
    }
}

fn lock_profile() -> Result<File, String> {
    let dir = eframe::storage_dir("easyPDF").ok_or("The easyPDF data directory is unavailable.")?;
    std::fs::create_dir_all(&dir).map_err(|_| "Could not prepare the easyPDF data directory for ChatGPT.")?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(dir.join("chatgpt-session.lock"))
        .map_err(|_| "Could not open the ChatGPT session lock.")?;
    file.try_lock()
        .map_err(|error| format!("Could not lock the ChatGPT session. Close any other active easyPDF ChatGPT operation and try again: {error}"))?;
    Ok(file)
}

fn read_credential(name: &str) -> Result<Option<Vec<u8>>, String> {
    match credential_for(name)?.get_secret() {
        Ok(value) => Ok(Some(value)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(format!("Could not read the ChatGPT credential: {error}")),
    }
}

fn save_profile(profile: &Profile) -> Result<(), String> {
    let bytes = serde_json::to_vec(profile).map_err(|_| "Could not encode the ChatGPT session.")?;
    credential_for(PROFILE)?.set_secret(&bytes).map_err(|error| format!("Could not protect the ChatGPT session: {error}"))
}

fn random<const N: usize>() -> Result<[u8; N], String> {
    let mut output = [0; N];
    rand::SecureRandom::fill(&rand::SystemRandom::new(), &mut output).map_err(|_| "Could not generate OAuth security parameters.".to_string())?;
    Ok(output)
}

fn host_id() -> Result<String, String> {
    if let Some(bytes) = read_credential(HOST)? {
        return String::from_utf8(bytes).map_err(|_| "The saved ChatGPT host identifier is unreadable.".into());
    }
    let host = format!("urn:uuid:{}", uuid::Builder::from_random_bytes(random()?).into_uuid());
    credential_for(HOST)?.set_secret(host.as_bytes()).map_err(|error| format!("Could not save the ChatGPT host identifier: {error}"))?;
    Ok(host)
}

struct SignIn {
    listener: TcpListener,
    state: String,
    nonce: String,
    verifier: String,
    redirect: String,
}

impl SignIn {
    fn new() -> Result<Self, String> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).map_err(|_| "Could not start the local ChatGPT callback.")?;
        let port = listener.local_addr().map_err(|_| "Could not determine the ChatGPT callback port.")?.port();
        listener.set_nonblocking(true).map_err(|_| "Could not configure the ChatGPT callback.")?;
        Ok(Self {
            listener,
            state: URL_SAFE_NO_PAD.encode(random::<32>()?),
            nonce: URL_SAFE_NO_PAD.encode(random::<32>()?),
            verifier: URL_SAFE_NO_PAD.encode(random::<64>()?),
            redirect: format!("http://127.0.0.1:{port}{CALLBACK}"),
        })
    }

    fn authorization_url(&self, host: &str, current_id: Option<&str>) -> String {
        let challenge = URL_SAFE_NO_PAD.encode(digest::digest(&digest::SHA256, self.verifier.as_bytes()).as_ref());
        let mut query = url::form_urlencoded::Serializer::new(String::new());
        query.extend_pairs([
            ("client_id", current_id.unwrap_or("dynamic_agent_client")),
            ("ext_agent_host_id", host),
            ("response_type", "code"),
            ("redirect_uri", self.redirect.as_str()),
            ("scope", SCOPES),
            ("resource", RESOURCE),
            ("state", self.state.as_str()),
            ("nonce", self.nonce.as_str()),
            ("code_challenge_method", "S256"),
            ("code_challenge", challenge.as_str()),
        ]);
        if current_id.is_none() {
            query.append_pair("agent_name_hint", "easyPDF");
        }
        format!("{AUTHORIZE}?{}", query.finish())
    }

    fn callback(&self, client: Option<&str>) -> Result<(String, String), String> {
        let deadline = Instant::now() + Duration::from_secs(180);
        while Instant::now() < deadline {
            match self.listener.accept() {
                Ok((mut stream, _)) => {
                    let outcome = callback_target(&mut stream).as_deref().and_then(|target| parse_callback(target, &self.state, client));
                    let (status, message) = match &outcome {
                        Some(Ok(_)) => ("200 OK", "Return to easyPDF to finish connecting ChatGPT."),
                        Some(Err(_)) => ("400 Bad Request", "Authorization did not complete. Return to easyPDF."),
                        None => ("404 Not Found", "No matching ChatGPT authorization."),
                    };
                    let _ = write!(
                        stream,
                        "HTTP/1.1 {status}\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{message}",
                        message.len()
                    );
                    if let Some(outcome) = outcome {
                        return outcome;
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(75)),
                Err(_) => return Err("The local ChatGPT callback was interrupted.".into()),
            }
        }
        Err("ChatGPT authorization timed out. Try again from Preferences.".into())
    }
}

fn callback_target(stream: &mut TcpStream) -> Option<String> {
    let timeout = Duration::from_secs(2);
    stream.set_read_timeout(Some(timeout)).ok()?;
    stream.set_write_timeout(Some(timeout)).ok()?;
    let deadline = Instant::now() + timeout;
    let mut buffer = Vec::new();
    let mut chunk = [0; 1024];
    while buffer.len() < 8192 && Instant::now() < deadline {
        let size = stream.read(&mut chunk).ok()?;
        if size == 0 {
            return None;
        }
        buffer.extend_from_slice(chunk.get(..size)?);
        let mut headers = [httparse::EMPTY_HEADER; 32];
        let mut request = httparse::Request::new(&mut headers);
        match request.parse(&buffer).ok()? {
            httparse::Status::Complete(_) if request.method == Some("GET") => return request.path.map(str::to_string),
            httparse::Status::Complete(_) => return None,
            httparse::Status::Partial => {}
        }
    }
    None
}

fn parse_callback(target: &str, state: &str, client: Option<&str>) -> Option<Result<(String, String), String>> {
    let (path, query) = target.split_once('?')?;
    if path != CALLBACK {
        return None;
    }
    let params: Vec<_> = url::form_urlencoded::parse(query.as_bytes()).collect();
    let single = |key: &str| {
        let mut matches = params.iter().filter(|(name, _)| name == key);
        let value = matches.next()?.1.as_ref();
        matches.next().is_none().then_some(value)
    };
    if single("state") != Some(state) {
        return None;
    }
    Some((|| {
        if ["error", "code", "client_id"].iter().any(|key| params.iter().filter(|(name, _)| name == *key).count() > 1) {
            return Err("ChatGPT returned an ambiguous authorization response.".into());
        }
        if single("error").is_some() {
            return Err("ChatGPT authorization was declined or could not be completed.".into());
        }
        let code = single("code").filter(|code| !code.is_empty()).ok_or("ChatGPT did not return an authorization code.")?;
        let issued = match (client, single("client_id")) {
            (Some(expected), None) => expected,
            (Some(expected), Some(issued)) if expected == issued => expected,
            (None, Some(issued)) if issued.starts_with("oaiapp_") && issued.len() <= 200 => issued,
            _ => return Err("ChatGPT returned an unexpected client identifier.".into()),
        };
        Ok((code.into(), issued.into()))
    })())
}

fn verify_identity(agent: &ureq::Agent, token: &str, client_id: &str, nonce: &str) -> Result<Identity, String> {
    let invalid = || "OpenAI returned an invalid ChatGPT identity token.".to_string();
    let (signed, encoded_signature) = token.rsplit_once('.').ok_or_else(invalid)?;
    let (header, payload) = signed.split_once('.').ok_or_else(invalid)?;
    let decode = |part: &str| URL_SAFE_NO_PAD.decode(part).map_err(|_| invalid());
    let header: Value = serde_json::from_slice(&decode(header)?).map_err(|_| invalid())?;
    if header.get("alg").and_then(Value::as_str) != Some("RS256") {
        return Err(invalid());
    }
    let kid = header.get("kid").and_then(Value::as_str).ok_or_else(invalid)?;
    let mut response = agent.get(JWKS).call().map_err(|error| network_error("Could not verify ChatGPT identity", &error))?;
    let keys: Value = read_json(&mut response)?;
    let key = keys
        .get("keys")
        .and_then(Value::as_array)
        .and_then(|keys| {
            keys.iter().find(|key| {
                key.get("kid").and_then(Value::as_str) == Some(kid)
                    && key.get("kty").and_then(Value::as_str) == Some("RSA")
                    && key.get("use").and_then(Value::as_str).is_none_or(|usage| usage == "sig")
                    && key.get("alg").and_then(Value::as_str).is_none_or(|algorithm| algorithm == "RS256")
            })
        })
        .ok_or_else(invalid)?;
    let n = decode(key.get("n").and_then(Value::as_str).ok_or_else(invalid)?)?;
    let e = decode(key.get("e").and_then(Value::as_str).ok_or_else(invalid)?)?;
    signature::RsaPublicKeyComponents { n, e }
        .verify(&signature::RSA_PKCS1_2048_8192_SHA256, signed.as_bytes(), &decode(encoded_signature)?)
        .map_err(|_| invalid())?;
    let claims: Identity = serde_json::from_slice(&decode(payload)?).map_err(|_| invalid())?;
    validate_claims(&claims, client_id, nonce, SystemTime::now().duration_since(UNIX_EPOCH).map_err(|_| invalid())?.as_secs())?;
    Ok(claims)
}

fn validate_claims(claims: &Identity, client_id: &str, nonce: &str, now: u64) -> Result<(), String> {
    let matches_audience = match &claims.aud {
        Value::String(audience) => audience == client_id,
        Value::Array(audiences) => {
            audiences.iter().any(|audience| audience.as_str() == Some(client_id))
                && (audiences.len() == 1 || claims.azp.as_deref() == Some(client_id))
        }
        _ => false,
    };
    if claims.iss != ISSUER
        || claims.sub.trim().is_empty()
        || !matches_audience
        || claims.azp.as_deref().is_some_and(|party| party != client_id)
        || claims.nonce != nonce
        || claims.exp <= now
        || claims.iat > now.saturating_add(5)
    {
        return Err("ChatGPT identity did not match this authorization.".into());
    }
    Ok(())
}

fn require_plan(scope: Option<&str>) -> Result<(), String> {
    if scope.is_some_and(|scope| scope.split_ascii_whitespace().any(|value| value == PLAN_SCOPE)) {
        return Ok(());
    }
    Err("ChatGPT did not grant access to your plan. Check eligibility and allow plan usage in the browser.".into())
}

fn cache_access(tokens: Tokens) -> (String, Instant) {
    let lifetime = tokens.expires_in.min(86_400).saturating_sub(60);
    (tokens.access_token, Instant::now() + Duration::from_secs(lifetime))
}

fn read_json<T: serde::de::DeserializeOwned>(response: &mut ureq::http::Response<ureq::Body>) -> Result<T, String> {
    let text = response.body_mut().with_config().limit(MAX_JSON).read_to_string().map_err(|_| "Could not read the OpenAI response.")?;
    serde_json::from_str(&text).map_err(|_| "OpenAI returned an invalid response.".into())
}

fn network_error(context: &str, error: &ureq::Error) -> String {
    match error {
        ureq::Error::StatusCode(status) => format!("{context} (HTTP {status})."),
        _ => format!("{context}. Check your connection and try again."),
    }
}

fn post_form(agent: &ureq::Agent, endpoint: &str, pairs: &[(&str, &str)]) -> Result<ureq::http::Response<ureq::Body>, ureq::Error> {
    let body = url::form_urlencoded::Serializer::new(String::new()).extend_pairs(pairs.iter().copied()).finish();
    agent.post(endpoint).header("Content-Type", "application/x-www-form-urlencoded").send(body)
}

fn token_request(agent: &ureq::Agent, pairs: &[(&str, &str)]) -> Result<Tokens, String> {
    let mut response = post_form(agent, TOKEN, pairs).map_err(|error| network_error("ChatGPT authorization or renewal failed", &error))?;
    let tokens: Tokens = read_json(&mut response)?;
    if tokens.access_token.is_empty() {
        return Err("OpenAI did not return a ChatGPT access token.".into());
    }
    Ok(tokens)
}

fn revoke(client_id: &str, refresh_token: &str) -> Result<(), String> {
    let warn = || "ChatGPT was disconnected locally. To revoke remote access, use your ChatGPT settings.".to_string();
    let agent = configured_agent(0).map_err(|_| warn())?;
    let mut response = agent.get(DISCOVERY).call().map_err(|_| warn())?;
    let metadata: Value = read_json(&mut response).map_err(|_| warn())?;
    let endpoint = metadata.get("revocation_endpoint").and_then(Value::as_str).ok_or_else(warn)?;
    let endpoint = Url::parse(endpoint).map_err(|_| warn())?;
    if endpoint.origin().ascii_serialization() != ISSUER || !endpoint.username().is_empty() || endpoint.password().is_some() {
        return Err(warn());
    }
    let response = post_form(&agent, endpoint.as_str(), &[("token", refresh_token), ("token_type_hint", "refresh_token"), ("client_id", client_id)])
        .map_err(|_| warn())?;
    if !response.status().is_success() {
        return Err(warn());
    }
    Ok(())
}

fn load_models(agent: &ureq::Agent, access: &str) -> Result<Vec<LocalChatGptModel>, String> {
    let mut response = agent
        .get(format!("{RESOURCE}/models"))
        .header("Authorization", format!("Bearer {access}"))
        .call()
        .map_err(|error| network_error("Could not load ChatGPT models", &error))?;
    let catalog: Value = read_json(&mut response)?;
    let models = catalog.get("models").and_then(Value::as_array).ok_or("OpenAI returned an invalid ChatGPT model list.")?;
    let models: Vec<_> = models
        .iter()
        .filter_map(|model| {
            if model.get("visibility")?.as_str()? != "list" {
                return None;
            }
            let slug = model.get("slug")?.as_str()?;
            let label = model.get("display_name").and_then(Value::as_str).filter(|label| !label.is_empty()).unwrap_or(slug);
            (!slug.is_empty() && slug.len() <= 160).then(|| LocalChatGptModel { slug: slug.into(), label: label.chars().take(160).collect() })
        })
        .take(500)
        .collect();
    if models.is_empty() {
        return Err("No models are available for this ChatGPT account.".into());
    }
    Ok(models)
}

fn parse_stream(stream: impl BufRead) -> Result<String, String> {
    let mut result = String::new();
    for line in stream.lines() {
        let line = line.map_err(|_| "Could not read the ChatGPT response.")?;
        let Some(data) = line.strip_prefix("data:").map(str::trim) else { continue };
        if data == "[DONE]" {
            break;
        }
        let event: Value = serde_json::from_str(data).map_err(|_| "OpenAI returned an invalid ChatGPT event.")?;
        match event.get("type").and_then(Value::as_str) {
            Some("response.output_text.delta") => {
                if let Some(delta) = event.get("delta").and_then(Value::as_str) {
                    result.push_str(delta);
                }
            }
            Some("response.completed") if !result.trim().is_empty() => return Ok(result.chars().take(200_000).collect()),
            Some("response.completed") => return Err("ChatGPT returned no text.".into()),
            Some("response.failed" | "response.incomplete" | "error") => {
                let code = event.pointer("/response/error/code").or_else(|| event.get("code")).and_then(Value::as_str);
                return Err(match code {
                    Some("subscription_sharing_usage_limit_exceeded") => "The ChatGPT plan usage limit has been reached.",
                    Some("subscription_sharing_usage_unavailable") => "ChatGPT plan usage is unavailable for this request.",
                    _ => "ChatGPT did not complete the request.",
                }
                .into());
            }
            _ => {}
        }
    }
    Err("The ChatGPT response ended before completion.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_configuration_uses_loopback_and_dynamic_registration() {
        let sign_in = SignIn::new().unwrap();
        let authorization = Url::parse(&sign_in.authorization_url("urn:uuid:example", None)).unwrap();
        let pairs: std::collections::BTreeMap<_, _> = authorization.query_pairs().collect();
        assert_eq!(pairs.get("client_id").map(|value| value.as_ref()), Some("dynamic_agent_client"));
        assert_eq!(pairs.get("code_challenge_method").map(|value| value.as_ref()), Some("S256"));
        let callback = Url::parse(pairs.get("redirect_uri").unwrap()).unwrap();
        assert_eq!(callback.host_str(), Some("127.0.0.1"));
        assert_eq!(callback.path(), CALLBACK);
        assert_eq!(callback.port(), Some(sign_in.listener.local_addr().unwrap().port()));
    }

    #[test]
    fn verified_claims_still_require_matching_nonce_and_plan_consent() {
        let claims = Identity {
            sub: "subject".into(),
            email: None,
            iss: ISSUER.into(),
            aud: json!("oaiapp_test"),
            azp: None,
            exp: 120,
            iat: 50,
            nonce: "nonce".into(),
        };
        assert!(validate_claims(&claims, "oaiapp_test", "nonce", 100).is_ok());
        assert!(validate_claims(&claims, "oaiapp_test", "different", 100).is_err());
        assert!(validate_claims(&claims, "oaiapp_other", "nonce", 100).is_err());
        assert!(validate_claims(&claims, "oaiapp_test", "nonce", 120).is_err());
        assert!(require_plan(Some(SCOPES)).is_ok());
        assert!(require_plan(Some("openid profile email")).is_err());
    }

    #[test]
    fn callback_binds_state_path_and_issued_client() {
        assert!(parse_callback("/auth/callback?state=wrong&code=a&client_id=oaiapp_test", "expected", None).is_none());
        assert!(parse_callback("/other?state=expected&code=a", "expected", None).is_none());
        assert!(parse_callback("/auth/callback?state=expected&state=expected&code=a", "expected", None).is_none());
        let callback = "/auth/callback?state=expected&code=a&client_id=oaiapp_test";
        assert!(matches!(parse_callback(callback, "expected", None), Some(Ok((code, id))) if code == "a" && id == "oaiapp_test"));
        assert!(matches!(parse_callback(callback, "expected", Some("oaiapp_other")), Some(Err(_))));
        assert!(matches!(parse_callback("/auth/callback?state=expected&error=access_denied", "expected", None), Some(Err(_))));
    }

    #[test]
    fn only_completed_inference_is_successful() {
        let delta = "data: {\"type\":\"response.output_text.delta\",\"delta\":\"Hello\"}\r\n\r\n";
        assert!(parse_stream(delta.as_bytes()).is_err());
        let completed = "data: {\"type\":\"response.completed\"}\r\n\r\n";
        assert_eq!(parse_stream(format!("{delta}{completed}").as_bytes()), Ok("Hello".into()));
        assert!(parse_stream(b"data: {\"type\":\"response.failed\"}\n\n".as_slice()).is_err());
    }
}
