(() => {
    'use strict';

    const cleanup = async () => {
        let failed = false;
        try {
            if ('caches' in window) {
                const names = await caches.keys();
                const results = await Promise.all(names.filter((name) => name.startsWith('easypdf-recent-')).map((name) => caches.delete(name)));
                failed ||= results.some((deleted) => !deleted);
            }
        } catch {
            failed = true;
        }
        try {
            const keys = [];
            for (let index = 0; index < localStorage.length; index += 1) {
                const key = localStorage.key(index);
                if (key && (key.startsWith('easypdf-user-') || key.startsWith('easypdf-session-'))) keys.push(key);
            }
            for (const key of keys) localStorage.removeItem(key);
        } catch {
            failed = true;
        }
        window.location.replace(failed ? '/?logout_cleanup=failed' : '/');
    };

    void cleanup();
})();
