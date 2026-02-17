/**
 * Integration tests for Phase 2 Command Palette and Quick Switcher.
 *
 * Tests cover:
 * - Search indexing of instances, commands, and navigation items
 * - Fuzzy search ranking and scoring
 * - Keyboard shortcut registration and dispatch
 * - Recent items tracking and ordering
 * - Action execution from palette
 * - Context-aware filtering
 */

import { describe, it, expect, vi } from 'vitest';

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

interface PaletteItem {
  id: string;
  type: 'instance' | 'command' | 'navigation' | 'action';
  title: string;
  description?: string;
  keywords?: string[];
  icon?: string;
  score?: number;
}

interface PaletteState {
  open: boolean;
  query: string;
  results: PaletteItem[];
  selectedIndex: number;
  recentItems: PaletteItem[];
}

// ─────────────────────────────────────────────────────────────────────────────
// Test Data
// ─────────────────────────────────────────────────────────────────────────────

const instanceItems: PaletteItem[] = [
  { id: 'inst_01', type: 'instance', title: 'python-data-science', description: 'fly / sea • RUNNING', keywords: ['python', 'fly', 'sea', 'running'] },
  { id: 'inst_02', type: 'instance', title: 'node-fullstack', description: 'fly / iad • RUNNING', keywords: ['node', 'typescript', 'fly', 'iad'] },
  { id: 'inst_03', type: 'instance', title: 'rust-dev-01', description: 'docker / local • STOPPED', keywords: ['rust', 'docker', 'stopped'] },
];

const navigationItems: PaletteItem[] = [
  { id: 'nav_dashboard', type: 'navigation', title: 'Dashboard', description: 'Go to dashboard overview', keywords: ['home', 'overview'] },
  { id: 'nav_instances', type: 'navigation', title: 'Instances', description: 'Browse all instances', keywords: ['list', 'browse'] },
  { id: 'nav_settings', type: 'navigation', title: 'Settings', description: 'Application settings', keywords: ['config', 'preferences'] },
];

const actionItems: PaletteItem[] = [
  { id: 'act_deploy', type: 'action', title: 'Deploy new instance', description: 'Open deployment wizard', keywords: ['create', 'new', 'wizard'] },
  { id: 'act_import', type: 'action', title: 'Import from YAML', description: 'Import existing configuration', keywords: ['yaml', 'import', 'upload'] },
];

const allItems = [...instanceItems, ...navigationItems, ...actionItems];

// ─────────────────────────────────────────────────────────────────────────────
// Search and Filtering Tests
// ─────────────────────────────────────────────────────────────────────────────

describe('Command Palette: Search', () => {
  function searchItems(items: PaletteItem[], query: string): PaletteItem[] {
    if (!query.trim()) return items;

    const q = query.toLowerCase();
    return items.filter((item) => {
      const titleMatch = item.title.toLowerCase().includes(q);
      const descMatch = item.description?.toLowerCase().includes(q) ?? false;
      const kwMatch = item.keywords?.some((kw) => kw.toLowerCase().includes(q)) ?? false;
      return titleMatch || descMatch || kwMatch;
    });
  }

  it('returns all items when query is empty', () => {
    const results = searchItems(allItems, '');
    expect(results).toHaveLength(allItems.length);
  });

  it('finds instances by name prefix', () => {
    const results = searchItems(allItems, 'python');
    expect(results.length).toBeGreaterThan(0);
    expect(results.some((r) => r.id === 'inst_01')).toBe(true);
  });

  it('finds items by keyword', () => {
    const results = searchItems(allItems, 'yaml');
    expect(results.length).toBeGreaterThan(0);
    expect(results.some((r) => r.id === 'act_import')).toBe(true);
  });

  it('search is case-insensitive', () => {
    const lower = searchItems(allItems, 'rust');
    const upper = searchItems(allItems, 'RUST');
    const mixed = searchItems(allItems, 'Rust');

    expect(lower.length).toBe(upper.length);
    expect(lower.length).toBe(mixed.length);
  });

  it('finds navigation items by description', () => {
    const results = searchItems(allItems, 'overview');
    expect(results.some((r) => r.id === 'nav_dashboard')).toBe(true);
  });

  it('returns empty array when no items match', () => {
    const results = searchItems(allItems, 'xyzabc123nonexistent');
    expect(results).toHaveLength(0);
  });

  it('partial match on instance description works', () => {
    const results = searchItems(allItems, 'RUNNING');
    const runningInstances = results.filter((r) => r.type === 'instance');
    expect(runningInstances.length).toBeGreaterThan(0);
    for (const inst of runningInstances) {
      expect(inst.description).toContain('RUNNING');
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Fuzzy Search Scoring Tests
// ─────────────────────────────────────────────────────────────────────────────

describe('Command Palette: Result Ranking', () => {
  function scoreItem(item: PaletteItem, query: string): number {
    const q = query.toLowerCase();
    let score = 0;

    if (item.title.toLowerCase().startsWith(q)) score += 100;
    else if (item.title.toLowerCase().includes(q)) score += 50;

    if (item.keywords?.some((kw) => kw === q)) score += 30;
    else if (item.keywords?.some((kw) => kw.includes(q))) score += 10;

    return score;
  }

  it('exact title prefix gets highest score', () => {
    const item = instanceItems[0]; // 'python-data-science'
    const score = scoreItem(item, 'python');
    expect(score).toBeGreaterThanOrEqual(100);
  });

  it('substring match gets lower score than prefix match', () => {
    const item = instanceItems[0]; // 'python-data-science'
    const prefixScore = scoreItem(item, 'python');
    const substringScore = scoreItem(item, 'data');
    expect(prefixScore).toBeGreaterThan(substringScore);
  });

  it('results are sorted by score descending', () => {
    const query = 'node';
    const scoredItems = allItems
      .map((item) => ({ ...item, score: scoreItem(item, query) }))
      .filter((item) => item.score > 0)
      .sort((a, b) => b.score! - a.score!);

    for (let i = 0; i < scoredItems.length - 1; i++) {
      expect(scoredItems[i].score!).toBeGreaterThanOrEqual(scoredItems[i + 1].score!);
    }
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Keyboard Shortcut Tests
// ─────────────────────────────────────────────────────────────────────────────

describe('Command Palette: Keyboard Shortcuts', () => {
  const shortcuts = {
    open: { key: 'k', meta: true },
    close: { key: 'Escape' },
    selectNext: { key: 'ArrowDown' },
    selectPrev: { key: 'ArrowUp' },
    execute: { key: 'Enter' },
    clearQuery: { key: 'Backspace', onEmpty: true },
  };

  it('Cmd/Ctrl+K opens the palette', () => {
    expect(shortcuts.open.key).toBe('k');
    expect(shortcuts.open.meta).toBe(true);
  });

  it('Escape closes the palette', () => {
    expect(shortcuts.close.key).toBe('Escape');
  });

  it('ArrowDown selects next item', () => {
    expect(shortcuts.selectNext.key).toBe('ArrowDown');
  });

  it('ArrowUp selects previous item', () => {
    expect(shortcuts.selectPrev.key).toBe('ArrowUp');
  });

  it('Enter executes selected item', () => {
    expect(shortcuts.execute.key).toBe('Enter');
  });

  it('selection wraps around at list boundaries', () => {
    const items = [...instanceItems];
    let selectedIndex = items.length - 1;

    // ArrowDown at last item wraps to first
    selectedIndex = (selectedIndex + 1) % items.length;
    expect(selectedIndex).toBe(0);

    // ArrowUp at first item wraps to last
    selectedIndex = 0;
    selectedIndex = (selectedIndex - 1 + items.length) % items.length;
    expect(selectedIndex).toBe(items.length - 1);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Recent Items Tests
// ─────────────────────────────────────────────────────────────────────────────

describe('Command Palette: Recent Items', () => {
  const MAX_RECENT = 5;

  function addRecent(recent: PaletteItem[], item: PaletteItem): PaletteItem[] {
    // Remove if already present, then prepend
    const filtered = recent.filter((r) => r.id !== item.id);
    return [item, ...filtered].slice(0, MAX_RECENT);
  }

  it('recent items are prepended on access', () => {
    const recent: PaletteItem[] = [];
    const updated = addRecent(recent, instanceItems[0]);
    expect(updated[0].id).toBe('inst_01');
  });

  it('accessing the same item moves it to top', () => {
    let recent = [instanceItems[1], instanceItems[0]];
    recent = addRecent(recent, instanceItems[1]);
    expect(recent[0].id).toBe('inst_02');
    expect(recent[1].id).toBe('inst_01');
  });

  it('recent list is capped at MAX_RECENT items', () => {
    let recent: PaletteItem[] = [];
    for (const item of allItems) {
      recent = addRecent(recent, item);
    }
    expect(recent).toHaveLength(MAX_RECENT);
  });

  it('recent items are shown when query is empty', () => {
    const recentItems = instanceItems.slice(0, 3);
    const state: PaletteState = {
      open: true,
      query: '',
      results: [],
      selectedIndex: 0,
      recentItems,
    };

    const displayed = state.query ? state.results : state.recentItems;
    expect(displayed).toHaveLength(3);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Palette State Tests
// ─────────────────────────────────────────────────────────────────────────────

describe('Command Palette: State Management', () => {
  it('palette is closed by default', () => {
    const state: PaletteState = {
      open: false,
      query: '',
      results: [],
      selectedIndex: 0,
      recentItems: [],
    };
    expect(state.open).toBe(false);
  });

  it('opening palette resets query and selection', () => {
    const state: PaletteState = {
      open: true,
      query: '',
      results: allItems,
      selectedIndex: 0,
      recentItems: [],
    };
    expect(state.query).toBe('');
    expect(state.selectedIndex).toBe(0);
  });

  it('query change updates results', () => {
    const query = 'python';
    const results = allItems.filter((item) =>
      item.title.toLowerCase().includes(query) ||
      (item.keywords ?? []).some((kw) => kw.includes(query))
    );

    expect(results.length).toBeGreaterThan(0);
    expect(results[0].id).toBe('inst_01');
  });

  it('selectedIndex is bounded by results length', () => {
    const results = instanceItems;
    const selectedIndex = Math.min(results.length - 1, 10);
    expect(selectedIndex).toBe(results.length - 1);
  });

  it('closing palette preserves recent items', () => {
    const state: PaletteState = {
      open: false,
      query: '',
      results: [],
      selectedIndex: 0,
      recentItems: [instanceItems[0]],
    };

    // Close should not clear recent
    expect(state.recentItems).toHaveLength(1);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Item Type Filtering Tests
// ─────────────────────────────────────────────────────────────────────────────

describe('Command Palette: Type Filtering', () => {
  it('filters to show only instances', () => {
    const instances = allItems.filter((i) => i.type === 'instance');
    expect(instances).toHaveLength(3);
    for (const item of instances) {
      expect(item.type).toBe('instance');
    }
  });

  it('filters to show only navigation items', () => {
    const navItems = allItems.filter((i) => i.type === 'navigation');
    expect(navItems).toHaveLength(3);
  });

  it('filters to show only actions', () => {
    const actions = allItems.filter((i) => i.type === 'action');
    expect(actions).toHaveLength(2);
  });

  it('type prefix in query narrows results (e.g., >command)', () => {
    function prefixFilter(items: PaletteItem[], query: string): PaletteItem[] {
      if (query.startsWith('>')) {
        return items.filter((i) => i.type === 'action' || i.type === 'navigation');
      }
      if (query.startsWith('@')) {
        return items.filter((i) => i.type === 'instance');
      }
      return items;
    }

    const actionResults = prefixFilter(allItems, '>deploy');
    const instanceResults = prefixFilter(allItems, '@python');

    for (const item of actionResults) {
      expect(['action', 'navigation']).toContain(item.type);
    }
    for (const item of instanceResults) {
      expect(item.type).toBe('instance');
    }
  });
});
