import { useState, useCallback, useRef } from "react";
import type { SearchAddon } from "@xterm/addon-search";

interface UseTerminalSearchOptions {
  searchAddon: SearchAddon | null;
}

export function useTerminalSearch({ searchAddon }: UseTerminalSearchOptions) {
  const [query, setQuery] = useState("");
  const [isOpen, setIsOpen] = useState(false);
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [wholeWord, setWholeWord] = useState(false);
  const [useRegex, setUseRegex] = useState(false);
  const lastQueryRef = useRef("");

  const search = useCallback(
    (searchQuery: string, direction: "next" | "prev" = "next") => {
      if (!searchAddon || !searchQuery) return;

      const options = {
        caseSensitive,
        wholeWord,
        regex: useRegex,
        decorations: {
          matchBackground: "#f6f17688",
          matchBorder: "#f6f176",
          matchOverviewRuler: "#f6f176",
          activeMatchBackground: "#ff9632",
          activeMatchBorder: "#ff9632",
          activeMatchColorOverviewRuler: "#ff9632",
        },
      };

      if (direction === "next") {
        searchAddon.findNext(searchQuery, options);
      } else {
        searchAddon.findPrevious(searchQuery, options);
      }

      lastQueryRef.current = searchQuery;
    },
    [searchAddon, caseSensitive, wholeWord, useRegex],
  );

  const clearSearch = useCallback(() => {
    searchAddon?.findNext("", {});
    setQuery("");
  }, [searchAddon]);

  const open = useCallback(() => setIsOpen(true), []);

  const close = useCallback(() => {
    setIsOpen(false);
    clearSearch();
  }, [clearSearch]);

  const handleQueryChange = useCallback(
    (newQuery: string) => {
      setQuery(newQuery);
      search(newQuery);
    },
    [search],
  );

  return {
    query,
    isOpen,
    caseSensitive,
    wholeWord,
    useRegex,
    setQuery: handleQueryChange,
    setCaseSensitive,
    setWholeWord,
    setUseRegex,
    searchNext: () => search(query, "next"),
    searchPrev: () => search(query, "prev"),
    open,
    close,
    clearSearch,
  };
}
