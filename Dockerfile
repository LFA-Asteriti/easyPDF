# syntax=docker/dockerfile:1.7

FROM rust:bookworm@sha256:e6f2cff35d0f309acf8433b4f05878ed77092602c5065cc81801177da0bb3a9c AS rust-with-fonts
ARG CRAFT_FONTS_REF=b076417b5880bd4154bd22f00695b1d28c6bea83
ARG CRAFT_I18N_REF=575cfdef63227b14daf173b8cc87928bfdfa325e
RUN git clone https://github.com/storytold/craft-fonts.git /craft-fonts \
    && git -C /craft-fonts checkout --detach "$CRAFT_FONTS_REF" \
    && git clone https://github.com/storytold/craft-i18n.git /craft-i18n \
    && git -C /craft-i18n checkout --detach "$CRAFT_I18N_REF"
ENV CRAFT_FONTS_DIR=/craft-fonts
ENV CRAFT_FONTS_REQUIRED=1
ENV CRAFT_I18N_DIR=/craft-i18n
ENV CRAFT_I18N_REQUIRED=1

FROM rust-with-fonts AS cli-builder
WORKDIR /build
COPY . .
RUN --mount=type=cache,id=easypdf-cargo-registry-cli,target=/usr/local/cargo/registry \
    --mount=type=cache,id=easypdf-cargo-git-cli,target=/usr/local/cargo/git \
    --mount=type=cache,id=easypdf-cli-target,target=/build/target \
    CARGO_NET_GIT_FETCH_WITH_CLI=true cargo build --release -p easypdf-cli \
    && cp /build/target/release/easypdf-cli /easypdf-cli

FROM rust-with-fonts AS web-builder
RUN --mount=type=cache,id=easypdf-cargo-registry,target=/usr/local/cargo/registry \
    rustup target add wasm32-unknown-unknown \
    && cargo install trunk --locked --version 0.21.14
WORKDIR /build
COPY . .
WORKDIR /build/apps/easypdf-web
RUN --mount=type=cache,id=easypdf-cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=easypdf-web-target,target=/build/target \
    --mount=type=cache,id=easypdf-trunk-tools,target=/root/.cache \
    touch /build/crates/engine/src/catalog.rs \
          /build/crates/ui-egui/src/lib.rs \
          /build/crates/ui-egui/src/connected.rs \
          /build/crates/ui-egui/src/dialogs.rs \
          /build/crates/ui-egui/src/panels.rs \
          /build/apps/easypdf-web/src/main.rs \
    && trunk build --release --public-url /editor/

FROM php:8.3-apache@sha256:a800bbd2c0aa322702259ed475556b2cbc311076a8d3c4bba9bfe96838745472
RUN apt-get update \
    && apt-get install -y --no-install-recommends libonig-dev libcurl4-openssl-dev \
    && docker-php-ext-install pdo_mysql mbstring curl \
    && a2enmod rewrite headers \
    && rm -rf /var/lib/apt/lists/*
ENV APACHE_DOCUMENT_ROOT=/var/www/html/public
RUN sed -ri 's!/var/www/html!${APACHE_DOCUMENT_ROOT}!g; s/AllowOverride None/AllowOverride All/g' /etc/apache2/sites-available/*.conf /etc/apache2/apache2.conf /etc/apache2/conf-available/*.conf
COPY web-portal/ /var/www/html/
# The documentation reuses the exact, already attributed Lucide icons shipped by the editor.
COPY assets/icons/ /var/www/html/public/assets/icons/
# Platform download cards use the pinned, attributed Devicon SVGs.
COPY assets/app-icon/Windows_11.svg assets/app-icon/Apple.svg assets/app-icon/Linux.svg /var/www/html/public/assets/platform/
# Contributor-original easyPDF screenshots used by the public homepage.
COPY assets/easypdf/easyPDF-home-1.jpg assets/easypdf/easyPDF-home-2.jpg assets/easypdf/easyPDF-home-3.jpg assets/easypdf/easyPDF-home-4.jpg /var/www/html/public/assets/home/
COPY web-portal/docker/php.ini /usr/local/etc/php/conf.d/easypdf.ini
COPY --from=web-builder /build/dist/web/ /var/www/html/editor-dist/
COPY --from=cli-builder /easypdf-cli /opt/easypdf/bin/easypdf-cli
RUN mkdir -p /var/www/easypdf-storage \
    && chown -R www-data:www-data /var/www/easypdf-storage /var/www/html \
    && chmod 0755 /var/www/html/docker/start.sh
EXPOSE 80
CMD ["/var/www/html/docker/start.sh"]
