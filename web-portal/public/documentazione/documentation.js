(() => {
    'use strict';

    const input = document.querySelector('#docs-search');
    const groups = [...document.querySelectorAll('[data-doc-group]')];
    const links = [...document.querySelectorAll('[data-doc-link]')];
    const empty = document.querySelector('#docs-empty');

    const normalize = (value) => value
        .normalize('NFD')
        .replace(/[\u0300-\u036f]/g, '')
        .toLocaleLowerCase('it')
        .trim();

    const filter = () => {
        const query = normalize(input?.value ?? '');
        let visible = 0;
        for (const group of groups) {
            const heading = normalize(group.querySelector('.docs-group-head h2')?.textContent ?? '');
            const match = query === '' || heading.startsWith(query);
            group.hidden = !match;
            const link = document.querySelector(`[data-doc-link="${CSS.escape(group.id)}"]`);
            if (link) link.hidden = !match;
            if (match) visible += 1;
        }
        for (const section of document.querySelectorAll('[data-doc-nav-section]')) {
            section.hidden = !section.querySelector('[data-doc-link]:not([hidden])');
        }
        if (empty) empty.hidden = visible !== 0;
    };

    input?.addEventListener('input', filter);
    input?.addEventListener('keydown', (event) => {
        if (event.key === 'Escape') {
            input.value = '';
            filter();
            input.blur();
        }
    });

    if ('IntersectionObserver' in window) {
        const observer = new IntersectionObserver((entries) => {
            const current = entries
                .filter((entry) => entry.isIntersecting && !entry.target.hidden)
                .sort((left, right) => left.boundingClientRect.top - right.boundingClientRect.top)[0];
            if (!current) return;
            for (const link of links) link.removeAttribute('aria-current');
            document.querySelector(`[data-doc-link="${CSS.escape(current.target.id)}"]`)?.setAttribute('aria-current', 'location');
        }, { rootMargin: '-12% 0px -76% 0px' });
        for (const group of groups) observer.observe(group);
    }
})();
