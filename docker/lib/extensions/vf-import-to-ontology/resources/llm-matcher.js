#!/usr/bin/env node

/**
 * LLM-based Semantic Matcher
 *
 * Uses Claude (via Z.AI if available) for fuzzy semantic matching
 * between content blocks and ontology concepts.
 */

const { spawn } = require('child_process');

/**
 * Call LLM for semantic matching
 * Tries Z.AI first (port 9600), falls back to direct prompt if needed
 */
async function llmSemanticMatch(blockContent, topCandidates, options = {}) {
  const { maxCandidates = 5, timeout = 10000 } = options;

  // Prepare the prompt
  const prompt = buildMatchingPrompt(blockContent, topCandidates);

  try {
    // Try Z.AI first (faster, cost-effective)
    const result = await callZAI(prompt, timeout);
    return parseMatchingResponse(result);
  } catch (error) {
    console.log('   ⚠️  Z.AI unavailable, using keyword-only matching');
    return null; // Fall back to keyword matching
  }
}

/**
 * Build prompt for semantic matching
 */
function buildMatchingPrompt(blockContent, candidates) {
  const candidateList = candidates
    .map((c, i) => `${i + 1}. ${c.concept.preferredTerm} (${c.concept.termId})
   Domain: ${c.concept.domain || 'general'}
   Definition: ${c.concept.definition?.substring(0, 150) || 'No definition'}
   Keywords: ${c.concept.keywords?.join(', ') || 'none'}
   Initial Score: ${(c.score * 100).toFixed(1)}%`)
    .join('\n\n');

  return `You are a semantic matching expert for an ontology system. Analyze this content block and determine which ontology concept it should be added to.

CONTENT BLOCK:
${blockContent.substring(0, 500)}${blockContent.length > 500 ? '...' : ''}

TOP CANDIDATE CONCEPTS:
${candidateList}

Based on semantic meaning, conceptual fit, and domain relevance, which concept (1-${candidates.length}) is the BEST match?

Respond with ONLY a JSON object:
{
  "match": <number 1-${candidates.length}>,
  "confidence": <0.0-1.0>,
  "reasoning": "<brief explanation>"
}`;
}

/**
 * Call Z.AI service (localhost:9600)
 */
function callZAI(prompt, timeout) {
  return new Promise((resolve, reject) => {
    const curl = spawn('curl', [
      '-X', 'POST',
      'http://localhost:9600/chat',
      '-H', 'Content-Type: application/json',
      '-d', JSON.stringify({
        prompt: prompt,
        timeout: timeout,
        max_tokens: 200
      }),
      '--max-time', Math.floor(timeout / 1000).toString()
    ]);

    let stdout = '';
    let stderr = '';

    curl.stdout.on('data', (data) => {
      stdout += data.toString();
    });

    curl.stderr.on('data', (data) => {
      stderr += data.toString();
    });

    curl.on('close', (code) => {
      if (code !== 0) {
        reject(new Error(`Z.AI call failed: ${stderr}`));
      } else {
        try {
          const response = JSON.parse(stdout);
          resolve(response.summary || response.response || stdout);
        } catch (e) {
          resolve(stdout);
        }
      }
    });

    setTimeout(() => {
      curl.kill();
      reject(new Error('Z.AI timeout'));
    }, timeout);
  });
}

/**
 * Parse LLM response to extract match decision
 */
function parseMatchingResponse(response) {
  try {
    // Try to extract JSON from response
    const jsonMatch = response.match(/\{[\s\S]*\}/);
    if (jsonMatch) {
      const parsed = JSON.parse(jsonMatch[0]);
      return {
        matchIndex: parsed.match - 1, // Convert 1-based to 0-based
        confidence: parsed.confidence,
        reasoning: parsed.reasoning
      };
    }
  } catch (e) {
    // Fall through to null
  }
  return null;
}

/**
 * Enhanced keyword extraction from block content
 * Extracts more semantic keywords than just the heading
 */
function extractSemanticKeywords(blockContent) {
  const text = blockContent.toLowerCase();

  // Extract words (4+ characters, excluding common words)
  const stopWords = new Set([
    'this', 'that', 'these', 'those', 'with', 'from', 'have', 'been',
    'will', 'would', 'could', 'should', 'about', 'after', 'before',
    'into', 'through', 'during', 'between', 'under', 'over', 'then',
    'when', 'where', 'what', 'which', 'while', 'there', 'their'
  ]);

  const words = text.match(/\b\w{4,}\b/g) || [];
  const keywords = words.filter(w => !stopWords.has(w));

  // Count frequency
  const freq = {};
  keywords.forEach(k => freq[k] = (freq[k] || 0) + 1);

  // Return top keywords sorted by frequency
  return Object.entries(freq)
    .sort((a, b) => b[1] - a[1])
    .slice(0, 30)
    .map(([word]) => word);
}

/**
 * Compute semantic similarity using ontology index keywords
 */
function computeSemanticScore(blockKeywords, conceptKeywords) {
  if (!conceptKeywords || conceptKeywords.length === 0) return 0;
  if (!blockKeywords || blockKeywords.length === 0) return 0;

  const blockSet = new Set(blockKeywords.map(k => k.toLowerCase()));
  const conceptSet = new Set(conceptKeywords.map(k => k.toLowerCase()));

  // Direct matches
  let directMatches = 0;
  for (const keyword of blockSet) {
    if (conceptSet.has(keyword)) {
      directMatches++;
    }
  }

  // Fuzzy matches (substring matches)
  let fuzzyMatches = 0;
  for (const blockKeyword of blockSet) {
    for (const conceptKeyword of conceptSet) {
      if (blockKeyword.includes(conceptKeyword) || conceptKeyword.includes(blockKeyword)) {
        fuzzyMatches += 0.5;
        break;
      }
    }
  }

  const totalMatches = directMatches + fuzzyMatches;
  const maxPossible = Math.min(blockSet.size, conceptSet.size);

  return maxPossible > 0 ? totalMatches / maxPossible : 0;
}

/**
 * Enhanced semantic targeting with LLM assistance
 */
async function findBestMatch(blockContent, index, options = {}) {
  const {
    useLLM = true,
    minConfidence = 0.15,
    topK = 5
  } = options;

  // Extract semantic keywords from block
  const blockKeywords = extractSemanticKeywords(blockContent);

  // Extract WikiLinks
  const wikiLinks = (blockContent.match(/\[\[([^\]]+)\]\]/g) || [])
    .map(link => link.slice(2, -2).trim());

  // Score all concepts
  const scored = [];
  for (const [term, concept] of Object.entries(index.concepts.concepts)) {
    let score = 0;

    // 1. Semantic keyword matching (70%)
    const keywordScore = computeSemanticScore(blockKeywords, concept.keywords);
    score += keywordScore * 0.7;

    // 2. WikiLink matching (20%)
    if (wikiLinks.length > 0 && concept.linksTo && concept.linksTo.length > 0) {
      const linkMatches = wikiLinks.filter(link =>
        concept.linksTo.some(target =>
          target.toLowerCase().includes(link.toLowerCase()) ||
          link.toLowerCase().includes(target.toLowerCase())
        )
      ).length;
      score += (linkMatches / wikiLinks.length) * 0.2;
    }

    // 3. Definition matching (10%)
    if (concept.definition) {
      const defWords = extractSemanticKeywords(concept.definition);
      const defScore = computeSemanticScore(blockKeywords, defWords);
      score += defScore * 0.1;
    }

    if (score > 0) {
      scored.push({ concept, score, term });
    }
  }

  // Sort by score and get top candidates
  scored.sort((a, b) => b.score - a.score);
  const topCandidates = scored.slice(0, topK);

  if (topCandidates.length === 0) {
    return null;
  }

  // If LLM is enabled and top score is ambiguous, use LLM for final decision
  if (useLLM && topCandidates.length > 1) {
    const topScore = topCandidates[0].score;
    const secondScore = topCandidates[1]?.score || 0;

    // If top 2 scores are close (within 20%), use LLM to decide
    if (secondScore > topScore * 0.8) {
      console.log('   🤖 Using LLM for ambiguous match...');
      const llmResult = await llmSemanticMatch(blockContent, topCandidates);

      if (llmResult && llmResult.matchIndex >= 0) {
        const selected = topCandidates[llmResult.matchIndex];
        return {
          concept: selected.concept,
          term: selected.term,
          score: Math.max(llmResult.confidence, selected.score), // Use higher of LLM or keyword score
          method: 'llm',
          reasoning: llmResult.reasoning
        };
      }
    }
  }

  // Return best keyword-based match
  const best = topCandidates[0];
  return {
    concept: best.concept,
    term: best.term,
    score: best.score,
    method: 'keyword',
    reasoning: `Keyword match: ${(best.score * 100).toFixed(1)}%`
  };
}

module.exports = {
  findBestMatch,
  extractSemanticKeywords,
  computeSemanticScore,
  llmSemanticMatch
};
