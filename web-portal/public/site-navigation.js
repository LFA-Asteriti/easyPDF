(() => {
    'use strict';

    const menus = [...document.querySelectorAll('[data-menu-toggle]')]
        .map((button) => ({ button, panel: document.getElementById(button.getAttribute('aria-controls') || '') }))
        .filter(({ panel }) => panel !== null);

    const close = ({ button, panel }, restoreFocus = false) => {
        if (button.getAttribute('aria-expanded') !== 'true') return;
        button.setAttribute('aria-expanded', 'false');
        panel.classList.remove('is-open');
        if (restoreFocus) button.focus();
    };

    for (const menu of menus) {
        menu.button.addEventListener('click', () => {
            const shouldOpen = menu.button.getAttribute('aria-expanded') !== 'true';
            for (const other of menus) close(other);
            if (shouldOpen) {
                menu.button.setAttribute('aria-expanded', 'true');
                menu.panel.classList.add('is-open');
            }
        });
        menu.panel.addEventListener('click', (event) => {
            if (event.target instanceof Element && event.target.closest('a')) close(menu);
        });
    }

    document.addEventListener('keydown', (event) => {
        if (event.key !== 'Escape') return;
        for (const menu of menus) close(menu, true);
    });
    document.addEventListener('click', (event) => {
        for (const menu of menus) {
            if (menu.button.contains(event.target) || menu.panel.contains(event.target)) continue;
            close(menu);
        }
    });
    window.addEventListener('resize', () => {
        if (window.innerWidth > 780) for (const menu of menus) close(menu);
    });
})();
