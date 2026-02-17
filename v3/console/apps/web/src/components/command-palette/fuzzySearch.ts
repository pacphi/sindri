/**
 * Lightweight fuzzy search implementation.
 * Scores a string against a query using a character-match heuristic.
 * Returns a score >= 0 (higher = better match), or -1 if no match.
 */
export function fuzzyScore(text: string, query: string): number {
  if (!query) return 0
  const lowerText = text.toLowerCase()
  const lowerQuery = query.toLowerCase()

  // Exact substring match gets highest score
  if (lowerText.includes(lowerQuery)) {
    return 1000 - lowerText.indexOf(lowerQuery)
  }

  // Fuzzy character-by-character match
  let queryIdx = 0
  let score = 0
  let consecutive = 0

  for (let i = 0; i < lowerText.length && queryIdx < lowerQuery.length; i++) {
    if (lowerText[i] === lowerQuery[queryIdx]) {
      queryIdx++
      consecutive++
      score += consecutive * 10
      // Bonus for matching at word boundaries
      if (i === 0 || lowerText[i - 1] === ' ' || lowerText[i - 1] === '-' || lowerText[i - 1] === '_') {
        score += 20
      }
    } else {
      consecutive = 0
    }
  }

  // All query characters must match
  if (queryIdx < lowerQuery.length) return -1
  // Penalize for longer text (prefer shorter, more specific matches)
  return score - lowerText.length
}

export function fuzzyMatch(text: string, query: string): boolean {
  return fuzzyScore(text, query) >= 0
}

export interface FuzzyResult<T> {
  item: T
  score: number
  field: string
}

export function fuzzySearch<T>(
  items: T[],
  query: string,
  getFields: (item: T) => string[],
): FuzzyResult<T>[] {
  if (!query.trim()) return items.map((item) => ({ item, score: 0, field: '' }))

  const results: FuzzyResult<T>[] = []

  for (const item of items) {
    const fields = getFields(item)
    let bestScore = -1
    let bestField = ''

    for (const field of fields) {
      const score = fuzzyScore(field, query)
      if (score > bestScore) {
        bestScore = score
        bestField = field
      }
    }

    if (bestScore >= 0) {
      results.push({ item, score: bestScore, field: bestField })
    }
  }

  return results.sort((a, b) => b.score - a.score)
}
