/**
 * Sindri FAQ - Interactive FAQ with Accordion and Search
 *
 * Features:
 * - Real-time full-text search with highlighting
 * - Accordion-style expandable Q&A items
 * - Category filtering
 * - Dark/light theme toggle with system preference detection
 * - Responsive design support
 * - Keyboard navigation (Enter/Space to toggle, Escape to close)
 */

// Configuration
const CONFIG = {
  dataUrl: 'v3-faq-data.json',
  githubBaseUrl: 'https://github.com/pacphi/sindri/blob/main/',
  debounceMs: 150,
  animationDurationMs: 300,
};

// State
const state = {
  faqData: null,
  filteredQuestions: [],
  activeCategory: 'all',
  activeVersion: 'all',
  searchQuery: '',
  openAccordions: new Set(),
};

// DOM Elements
const elements = {
  faqContainer: null,
  searchInput: null,
  clearSearch: null,
  searchStats: null,
  resultCount: null,
  categoryFilters: null,
  noResults: null,
  loading: null,
  themeToggle: null,
  resetSearch: null,
  versionFilters: null,
};

// ============================================================================
// Theme Management
// ============================================================================

function initTheme() {
  const savedTheme = localStorage.getItem('sindri-faq-theme');
  const systemPrefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;

  if (savedTheme === 'dark' || (!savedTheme && systemPrefersDark)) {
    document.documentElement.classList.add('dark');
  } else {
    document.documentElement.classList.remove('dark');
  }
}

function toggleTheme() {
  const isDark = document.documentElement.classList.toggle('dark');
  localStorage.setItem('sindri-faq-theme', isDark ? 'dark' : 'light');
}

// ============================================================================
// Data Loading
// ============================================================================

async function loadFaqData() {
  try {
    const response = await fetch(CONFIG.dataUrl);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);

    state.faqData = await response.json();

    // Ensure backward compatibility with new schema fields
    if (!state.faqData.personas) state.faqData.personas = [];
    if (!state.faqData.useCases) state.faqData.useCases = [];
    if (!state.faqData.meta) state.faqData.meta = {};

    state.filteredQuestions = [...state.faqData.questions];

    initVersionFilter();
    renderCategoryFilters();
    renderFaqItems();
    hideLoading();

  } catch (error) {
    console.error('Failed to load FAQ data:', error);
    showError('Failed to load FAQ data. Please refresh the page.');
  }
}

function hideLoading() {
  if (elements.loading) {
    elements.loading.style.display = 'none';
  }
}

function showError(message) {
  if (elements.loading) {
    elements.loading.innerHTML = `
      <div class="text-red-500 dark:text-red-400 flex items-center gap-2">
        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"/>
        </svg>
        <span>${message}</span>
      </div>
    `;
  }
}

// ============================================================================
// Category Filtering
// ============================================================================

function renderCategoryFilters() {
  if (!elements.categoryFilters || !state.faqData) return;

  const categoryButtons = state.faqData.categories.map(cat => `
    <button data-category="${cat.id}"
            class="category-pill px-4 py-2 rounded-full text-sm font-medium transition-all
                   bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-300
                   hover:bg-slate-200 dark:hover:bg-slate-700">
      ${cat.name}
      <span class="ml-1 text-xs opacity-60">(${getCategoryCount(cat.id)})</span>
    </button>
  `).join('');

  // Keep "All Categories" button, add category buttons
  const allButton = elements.categoryFilters.querySelector('[data-category="all"]');
  if (allButton) {
    allButton.innerHTML = `All <span class="ml-1 text-xs opacity-75">(${getTotalCount()})</span>`;
  }
  elements.categoryFilters.insertAdjacentHTML('beforeend', categoryButtons);

  // Add event listeners
  elements.categoryFilters.querySelectorAll('.category-pill').forEach(btn => {
    btn.addEventListener('click', () => setActiveCategory(btn.dataset.category));
  });
}

function getCategoryCount(categoryId) {
  let questions = state.faqData.questions.filter(q => q.category === categoryId);
  if (state.activeVersion !== 'all') {
    questions = questions.filter(q =>
      q.versionsApplicable && q.versionsApplicable.includes(state.activeVersion)
    );
  }
  return questions.length;
}

function getTotalCount() {
  if (state.activeVersion === 'all') return state.faqData.questions.length;
  return state.faqData.questions.filter(q =>
    q.versionsApplicable && q.versionsApplicable.includes(state.activeVersion)
  ).length;
}

function setActiveCategory(categoryId) {
  state.activeCategory = categoryId;

  // Update UI
  elements.categoryFilters.querySelectorAll('.category-pill').forEach(btn => {
    const isActive = btn.dataset.category === categoryId;
    btn.classList.toggle('active', isActive);
    btn.classList.toggle('bg-sindri-500', isActive);
    btn.classList.toggle('text-white', isActive);
    btn.classList.toggle('shadow-lg', isActive);
    btn.classList.toggle('shadow-sindri-500/25', isActive);
    btn.classList.toggle('bg-slate-100', !isActive);
    btn.classList.toggle('dark:bg-slate-800', !isActive);
    btn.classList.toggle('text-slate-600', !isActive);
    btn.classList.toggle('dark:text-slate-300', !isActive);
  });

  filterAndRender();
}

// ============================================================================
// Search Functionality
// ============================================================================

function initSearch() {
  if (!elements.searchInput) return;

  let debounceTimer;

  elements.searchInput.addEventListener('input', (e) => {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      state.searchQuery = e.target.value.trim().toLowerCase();
      updateClearButton();
      filterAndRender();
    }, CONFIG.debounceMs);
  });

  elements.searchInput.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      clearSearch();
    }
  });

  if (elements.clearSearch) {
    elements.clearSearch.addEventListener('click', clearSearch);
  }

  if (elements.resetSearch) {
    elements.resetSearch.addEventListener('click', () => {
      clearSearch();
      setActiveVersion('all');
      setActiveCategory('all');
    });
  }
}

function clearSearch() {
  state.searchQuery = '';
  if (elements.searchInput) {
    elements.searchInput.value = '';
  }
  updateClearButton();
  filterAndRender();
}

function updateClearButton() {
  if (elements.clearSearch) {
    elements.clearSearch.classList.toggle('hidden', !state.searchQuery);
  }
}

function filterAndRender() {
  if (!state.faqData) return;

  let filtered = state.faqData.questions;

  // Filter by version
  if (state.activeVersion !== 'all') {
    filtered = filtered.filter(q =>
      q.versionsApplicable && q.versionsApplicable.includes(state.activeVersion)
    );
  }

  // Filter by category
  if (state.activeCategory !== 'all') {
    filtered = filtered.filter(q => q.category === state.activeCategory);
  }

  // Filter by search query
  if (state.searchQuery) {
    const query = state.searchQuery.toLowerCase();
    filtered = filtered.filter(q => {
      const searchableText = [
        q.question,
        q.answer,
        ...q.tags,
        ...(q.keywords || []),
        getCategoryName(q.category)
      ].join(' ').toLowerCase();

      return searchableText.includes(query);
    });
  }

  state.filteredQuestions = filtered;
  renderFaqItems();
  updateSearchStats();
}

function updateSearchStats() {
  if (elements.searchStats && elements.resultCount) {
    if (state.searchQuery) {
      elements.searchStats.classList.remove('hidden');
      elements.resultCount.textContent = state.filteredQuestions.length;
    } else {
      elements.searchStats.classList.add('hidden');
    }
  }

  // Show/hide no results message
  if (elements.noResults) {
    elements.noResults.classList.toggle('hidden', state.filteredQuestions.length > 0);
  }
}

// ============================================================================
// FAQ Rendering
// ============================================================================

function renderFaqItems() {
  if (!elements.faqContainer || !state.faqData) return;

  // Clear existing items (except loading)
  const existingItems = elements.faqContainer.querySelectorAll('.faq-item');
  existingItems.forEach(item => item.remove());

  if (state.filteredQuestions.length === 0) {
    return;
  }

  // Group by category if not searching and showing all
  const shouldGroup = !state.searchQuery && state.activeCategory === 'all';

  if (shouldGroup) {
    renderGroupedFaq();
  } else {
    renderFlatFaq();
  }
}

function renderGroupedFaq() {
  const groupedQuestions = {};

  state.filteredQuestions.forEach(q => {
    if (!groupedQuestions[q.category]) {
      groupedQuestions[q.category] = [];
    }
    groupedQuestions[q.category].push(q);
  });

  // Render each category group
  state.faqData.categories.forEach(cat => {
    const questions = groupedQuestions[cat.id];
    if (!questions || questions.length === 0) return;

    const groupHtml = `
      <div class="faq-item mb-8">
        <h3 class="flex items-center gap-3 text-lg font-bold text-slate-900 dark:text-white mb-4 pb-2 border-b border-slate-200 dark:border-slate-700">
          ${getCategoryIcon(cat.icon)}
          <span>${cat.name}</span>
          <span class="text-sm font-normal text-slate-400">(${questions.length})</span>
        </h3>
        <div class="space-y-3">
          ${questions.map(q => createFaqItemHtml(q)).join('')}
        </div>
      </div>
    `;

    elements.faqContainer.insertAdjacentHTML('beforeend', groupHtml);
  });

  attachAccordionListeners();
}

function renderFlatFaq() {
  const html = state.filteredQuestions.map(q => createFaqItemHtml(q)).join('');
  elements.faqContainer.insertAdjacentHTML('beforeend', `<div class="faq-item space-y-3">${html}</div>`);
  attachAccordionListeners();
}

function createFaqItemHtml(question) {
  const isOpen = state.openAccordions.has(question.id);
  const highlightedQuestion = highlightText(question.question, state.searchQuery);
  const highlightedAnswer = highlightText(question.answer, state.searchQuery);

  return `
    <div class="accordion-item bg-white dark:bg-slate-800/50 rounded-xl border border-slate-200 dark:border-slate-700/50 shadow-sm hover:shadow-md transition-shadow overflow-hidden"
         data-id="${question.id}">
      <button class="accordion-trigger w-full px-5 py-4 text-left flex items-start gap-4 focus:outline-none focus:ring-2 focus:ring-inset focus:ring-sindri-500"
              aria-expanded="${isOpen}"
              aria-controls="content-${question.id}">
        <span class="accordion-icon mt-1 flex-shrink-0 w-5 h-5 rounded-full bg-sindri-100 dark:bg-sindri-900/50 text-sindri-600 dark:text-sindri-400 flex items-center justify-center transition-transform duration-300 ${isOpen ? 'rotate-180' : ''}">
          <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M19 9l-7 7-7-7"/>
          </svg>
        </span>
        <div class="flex-grow min-w-0">
          <div class="flex items-center gap-2 flex-wrap">
            <h4 class="text-base font-semibold text-slate-900 dark:text-white leading-snug">
              ${highlightedQuestion}
            </h4>
            ${renderVersionBadges(question.versionsApplicable)}
          </div>
        </div>
      </button>
      <div id="content-${question.id}"
           class="accordion-content ${isOpen ? 'open' : ''}"
           role="region"
           aria-labelledby="trigger-${question.id}">
        <div class="px-5 pb-5 pt-0 ml-9">
          <div class="prose prose-sm dark:prose-invert max-w-none text-slate-600 dark:text-slate-300 leading-relaxed">
            ${highlightedAnswer}
          </div>
          ${renderDocLinks(question.docs)}
          ${renderTags(question.tags)}
        </div>
      </div>
    </div>
  `;
}

function renderDocLinks(docs) {
  if (!docs || docs.length === 0) return '';

  const links = docs.map(doc => {
    const url = CONFIG.githubBaseUrl + doc;
    const name = doc.split('/').pop();
    return `<a href="${url}" target="_blank" rel="noopener noreferrer"
               class="inline-flex items-center gap-1 text-sindri-600 dark:text-sindri-400 hover:underline">
              <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"/>
              </svg>
              ${name}
            </a>`;
  }).join('');

  return `
    <div class="mt-4 pt-3 border-t border-slate-100 dark:border-slate-700/50">
      <span class="text-xs font-medium text-slate-400 dark:text-slate-500 uppercase tracking-wide">Documentation:</span>
      <div class="mt-1 flex flex-wrap gap-3 text-sm">
        ${links}
      </div>
    </div>
  `;
}

function renderTags(tags) {
  if (!tags || tags.length === 0) return '';

  return `
    <div class="mt-3 flex flex-wrap gap-1.5">
      ${tags.map(tag => `
        <span class="px-2 py-0.5 bg-slate-100 dark:bg-slate-700/50 text-slate-500 dark:text-slate-400 rounded text-xs">
          #${tag}
        </span>
      `).join('')}
    </div>
  `;
}

// ============================================================================
// Accordion Behavior
// ============================================================================

function attachAccordionListeners() {
  const triggers = elements.faqContainer.querySelectorAll('.accordion-trigger');

  triggers.forEach(trigger => {
    trigger.addEventListener('click', handleAccordionClick);
    trigger.addEventListener('keydown', handleAccordionKeydown);
  });
}

function handleAccordionClick(e) {
  const item = e.currentTarget.closest('.accordion-item');
  if (!item) return;

  const id = item.dataset.id;
  toggleAccordion(id, item);
}

function handleAccordionKeydown(e) {
  if (e.key === 'Enter' || e.key === ' ') {
    e.preventDefault();
    handleAccordionClick(e);
  }
}

function toggleAccordion(id, item) {
  const isOpen = state.openAccordions.has(id);
  const content = item.querySelector('.accordion-content');
  const icon = item.querySelector('.accordion-icon');
  const trigger = item.querySelector('.accordion-trigger');

  if (isOpen) {
    state.openAccordions.delete(id);
    content.classList.remove('open');
    icon.classList.remove('rotate-180');
    trigger.setAttribute('aria-expanded', 'false');
  } else {
    state.openAccordions.add(id);
    content.classList.add('open');
    icon.classList.add('rotate-180');
    trigger.setAttribute('aria-expanded', 'true');
  }
}

// ============================================================================
// Version Filtering
// ============================================================================

function initVersionFilter() {
  elements.versionFilters = document.getElementById('version-filters');
  if (!elements.versionFilters) return;

  elements.versionFilters.querySelectorAll('.version-toggle').forEach(btn => {
    btn.addEventListener('click', () => setActiveVersion(btn.dataset.version));
  });
}

function setActiveVersion(version) {
  state.activeVersion = version;

  // Update version toggle UI
  if (elements.versionFilters) {
    elements.versionFilters.querySelectorAll('.version-toggle').forEach(btn => {
      const isActive = btn.dataset.version === version;
      btn.classList.toggle('bg-sindri-500', isActive);
      btn.classList.toggle('text-white', isActive);
      btn.classList.toggle('shadow-lg', isActive);
      btn.classList.toggle('shadow-sindri-500/25', isActive);
      btn.classList.toggle('bg-slate-100', !isActive);
      btn.classList.toggle('dark:bg-slate-800', !isActive);
      btn.classList.toggle('text-slate-600', !isActive);
      btn.classList.toggle('dark:text-slate-300', !isActive);
    });
  }

  // Re-render category counts with version filter applied
  updateCategoryCounts();
  filterAndRender();
}

function updateCategoryCounts() {
  if (!elements.categoryFilters || !state.faqData) return;

  // Update "All" button count
  const allButton = elements.categoryFilters.querySelector('[data-category="all"]');
  if (allButton) {
    allButton.innerHTML = `All <span class="ml-1 text-xs opacity-75">(${getTotalCount()})</span>`;
  }

  // Update each category count
  state.faqData.categories.forEach(cat => {
    const btn = elements.categoryFilters.querySelector(`[data-category="${cat.id}"]`);
    if (btn) {
      const count = getCategoryCount(cat.id);
      const countSpan = btn.querySelector('span');
      if (countSpan) {
        countSpan.textContent = `(${count})`;
      }
    }
  });
}

function renderVersionBadges(versionsApplicable) {
  if (!versionsApplicable || versionsApplicable.length === 0) return '';
  return versionsApplicable.map(v => {
    const colors = v === 'v2'
      ? 'bg-amber-100 text-amber-800 dark:bg-amber-900/50 dark:text-amber-300'
      : 'bg-emerald-100 text-emerald-800 dark:bg-emerald-900/50 dark:text-emerald-300';
    return `<span class="px-1.5 py-0.5 rounded text-xs font-medium ${colors} whitespace-nowrap">${v.toUpperCase()}</span>`;
  }).join(' ');
}

// ============================================================================
// Utility Functions
// ============================================================================

function getCategoryName(categoryId) {
  if (!state.faqData) return categoryId;
  const cat = state.faqData.categories.find(c => c.id === categoryId);
  return cat ? cat.name : categoryId;
}

function getCategoryIcon(iconName) {
  const icons = {
    rocket: `<svg class="w-5 h-5 text-emerald-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
            </svg>`,
    cog: `<svg class="w-5 h-5 text-blue-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"/>
          </svg>`,
    cloud: `<svg class="w-5 h-5 text-purple-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 15a4 4 0 004 4h9a5 5 0 10-.1-9.999 5.002 5.002 0 10-9.78 2.096A4.001 4.001 0 003 15z"/>
            </svg>`,
    puzzle: `<svg class="w-5 h-5 text-orange-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 4a2 2 0 114 0v1a1 1 0 001 1h3a1 1 0 011 1v3a1 1 0 01-1 1h-1a2 2 0 100 4h1a1 1 0 011 1v3a1 1 0 01-1 1h-3a1 1 0 01-1-1v-1a2 2 0 10-4 0v1a1 1 0 01-1 1H7a1 1 0 01-1-1v-3a1 1 0 00-1-1H4a2 2 0 110-4h1a1 1 0 001-1V7a1 1 0 011-1h3a1 1 0 001-1V4z"/>
            </svg>`,
    key: `<svg class="w-5 h-5 text-red-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"/>
          </svg>`,
    wrench: `<svg class="w-5 h-5 text-amber-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"/>
            </svg>`,
    layers: `<svg class="w-5 h-5 text-indigo-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"/>
            </svg>`,
    refresh: `<svg class="w-5 h-5 text-cyan-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"/>
            </svg>`,
    'cloud-upload': `<svg class="w-5 h-5 text-purple-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"/>
            </svg>`,
    lock: `<svg class="w-5 h-5 text-red-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z"/>
          </svg>`,
    server: `<svg class="w-5 h-5 text-teal-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 12h14M5 12a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v4a2 2 0 01-2 2M5 12a2 2 0 00-2 2v4a2 2 0 002 2h14a2 2 0 002-2v-4a2 2 0 00-2-2m-2-4h.01M17 16h.01"/>
            </svg>`,
    shield: `<svg class="w-5 h-5 text-sky-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z"/>
            </svg>`,
    kubernetes: `<svg class="w-5 h-5 text-violet-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"/>
            </svg>`,
    stethoscope: `<svg class="w-5 h-5 text-lime-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z"/>
            </svg>`,
    'arrow-right': `<svg class="w-5 h-5 text-pink-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 7l5 5m0 0l-5 5m5-5H6"/>
            </svg>`,
  };

  return icons[iconName] || icons.cog;
}

function highlightText(text, query) {
  if (!query || query.length < 2) return text;

  try {
    const escapedQuery = query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const regex = new RegExp(`(${escapedQuery})`, 'gi');
    return text.replace(regex, '<mark class="highlight">$1</mark>');
  } catch (e) {
    return text;
  }
}

// ============================================================================
// Initialization
// ============================================================================

function initElements() {
  elements.faqContainer = document.getElementById('faq-container');
  elements.searchInput = document.getElementById('search-input');
  elements.clearSearch = document.getElementById('clear-search');
  elements.searchStats = document.getElementById('search-stats');
  elements.resultCount = document.getElementById('result-count');
  elements.categoryFilters = document.getElementById('category-filters');
  elements.noResults = document.getElementById('no-results');
  elements.loading = document.getElementById('loading');
  elements.themeToggle = document.getElementById('theme-toggle');
  elements.resetSearch = document.getElementById('reset-search');
  elements.versionFilters = document.getElementById('version-filters');
}

function init() {
  initElements();
  initTheme();
  initSearch();

  // Theme toggle
  if (elements.themeToggle) {
    elements.themeToggle.addEventListener('click', toggleTheme);
  }

  // System theme change listener
  window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
    if (!localStorage.getItem('sindri-faq-theme')) {
      document.documentElement.classList.toggle('dark', e.matches);
    }
  });

  // Load FAQ data
  loadFaqData();
}

// Start the app
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', init);
} else {
  init();
}
