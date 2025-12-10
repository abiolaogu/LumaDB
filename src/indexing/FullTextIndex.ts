/**
 * TDB+ Full-Text Index
 *
 * An inverted index for full-text search with support for:
 * - Tokenization
 * - Stop words filtering
 * - Stemming (basic)
 * - Fuzzy matching
 * - Relevance scoring
 */

import { DocumentId, DocumentData, ComparisonOperator, IndexStats } from '../types';

interface TokenInfo {
  documents: Map<DocumentId, number[]>; // documentId -> positions
  frequency: number;
}

export class FullTextIndex {
  private name: string;
  private fields: string[];
  private language: string;
  private stopWords: Set<string>;
  private tokens: Map<string, TokenInfo>;
  private documentLengths: Map<DocumentId, number>;
  private size: number;
  private queryCount: number;
  private hitCount: number;

  private static DEFAULT_STOP_WORDS = new Set([
    'a', 'an', 'and', 'are', 'as', 'at', 'be', 'by', 'for', 'from',
    'has', 'have', 'he', 'in', 'is', 'it', 'its', 'of', 'on', 'or',
    'she', 'that', 'the', 'they', 'this', 'to', 'was', 'were', 'will',
    'with', 'you', 'your',
  ]);

  constructor(
    name: string,
    fields: string[],
    language: string = 'en',
    stopWords?: string[]
  ) {
    this.name = name;
    this.fields = fields;
    this.language = language;
    this.stopWords = stopWords
      ? new Set(stopWords.map((w) => w.toLowerCase()))
      : FullTextIndex.DEFAULT_STOP_WORDS;
    this.tokens = new Map();
    this.documentLengths = new Map();
    this.size = 0;
    this.queryCount = 0;
    this.hitCount = 0;
  }

  /**
   * Insert a document into the index
   */
  insert(documentId: DocumentId, data: DocumentData): void {
    const text = this.extractText(data);
    if (!text) return;

    const tokens = this.tokenize(text);
    this.documentLengths.set(documentId, tokens.length);

    tokens.forEach((token, position) => {
      if (!this.tokens.has(token)) {
        this.tokens.set(token, { documents: new Map(), frequency: 0 });
      }

      const tokenInfo = this.tokens.get(token)!;
      if (!tokenInfo.documents.has(documentId)) {
        tokenInfo.documents.set(documentId, []);
        tokenInfo.frequency++;
        this.size++;
      }

      tokenInfo.documents.get(documentId)!.push(position);
    });
  }

  /**
   * Remove a document from the index
   */
  remove(documentId: DocumentId, data: DocumentData): void {
    this.documentLengths.delete(documentId);

    for (const [token, info] of this.tokens) {
      if (info.documents.has(documentId)) {
        info.documents.delete(documentId);
        info.frequency--;
        this.size--;

        if (info.documents.size === 0) {
          this.tokens.delete(token);
        }
      }
    }
  }

  /**
   * Query the index
   */
  query(operator: ComparisonOperator, value: any): DocumentId[] {
    this.queryCount++;

    const searchText = String(value);

    switch (operator) {
      case 'CONTAINS':
      case 'LIKE':
      case '=':
        return this.search(searchText);

      case 'MATCHES':
        return this.searchRegex(searchText);

      case 'STARTS WITH':
        return this.searchPrefix(searchText);

      default:
        return [];
    }
  }

  /**
   * Search for documents containing the query terms
   */
  search(query: string): DocumentId[] {
    const queryTokens = this.tokenize(query);
    if (queryTokens.length === 0) return [];

    // Find documents containing all tokens
    const documentScores = new Map<DocumentId, number>();

    for (const token of queryTokens) {
      const tokenInfo = this.tokens.get(token);
      if (!tokenInfo) continue;

      for (const [docId, positions] of tokenInfo.documents) {
        const score = this.calculateScore(docId, token, positions.length);
        documentScores.set(docId, (documentScores.get(docId) || 0) + score);
      }
    }

    if (documentScores.size > 0) {
      this.hitCount++;
    }

    // Sort by score descending
    return Array.from(documentScores.entries())
      .sort((a, b) => b[1] - a[1])
      .map(([docId]) => docId);
  }

  /**
   * Search with phrase matching
   */
  searchPhrase(phrase: string): DocumentId[] {
    const tokens = this.tokenize(phrase);
    if (tokens.length === 0) return [];

    // Find documents containing the first token
    const firstTokenInfo = this.tokens.get(tokens[0]);
    if (!firstTokenInfo) return [];

    const candidates = Array.from(firstTokenInfo.documents.entries());
    const results: DocumentId[] = [];

    for (const [docId, positions] of candidates) {
      // Check if the phrase appears in sequence
      for (const startPos of positions) {
        let found = true;
        for (let i = 1; i < tokens.length; i++) {
          const tokenInfo = this.tokens.get(tokens[i]);
          if (!tokenInfo) {
            found = false;
            break;
          }
          const docPositions = tokenInfo.documents.get(docId);
          if (!docPositions || !docPositions.includes(startPos + i)) {
            found = false;
            break;
          }
        }
        if (found) {
          results.push(docId);
          break;
        }
      }
    }

    if (results.length > 0) {
      this.hitCount++;
    }

    return results;
  }

  /**
   * Search for prefix matches
   */
  searchPrefix(prefix: string): DocumentId[] {
    const normalizedPrefix = this.normalize(prefix);
    const results = new Set<DocumentId>();

    for (const [token, info] of this.tokens) {
      if (token.startsWith(normalizedPrefix)) {
        for (const docId of info.documents.keys()) {
          results.add(docId);
        }
      }
    }

    if (results.size > 0) {
      this.hitCount++;
    }

    return Array.from(results);
  }

  /**
   * Search with regex pattern
   */
  searchRegex(pattern: string): DocumentId[] {
    let regex: RegExp;
    try {
      regex = new RegExp(pattern, 'i');
    } catch {
      return [];
    }

    const results = new Set<DocumentId>();

    for (const [token, info] of this.tokens) {
      if (regex.test(token)) {
        for (const docId of info.documents.keys()) {
          results.add(docId);
        }
      }
    }

    if (results.size > 0) {
      this.hitCount++;
    }

    return Array.from(results);
  }

  /**
   * Fuzzy search using Levenshtein distance
   */
  searchFuzzy(term: string, maxDistance: number = 2): DocumentId[] {
    const normalizedTerm = this.normalize(term);
    const results = new Map<DocumentId, number>();

    for (const [token, info] of this.tokens) {
      const distance = this.levenshteinDistance(normalizedTerm, token);
      if (distance <= maxDistance) {
        const score = 1 - distance / Math.max(normalizedTerm.length, token.length);
        for (const docId of info.documents.keys()) {
          results.set(docId, Math.max(results.get(docId) || 0, score));
        }
      }
    }

    if (results.size > 0) {
      this.hitCount++;
    }

    return Array.from(results.entries())
      .sort((a, b) => b[1] - a[1])
      .map(([docId]) => docId);
  }

  /**
   * Clear the index
   */
  clear(): void {
    this.tokens.clear();
    this.documentLengths.clear();
    this.size = 0;
  }

  /**
   * Get index statistics
   */
  getStats(): IndexStats {
    return {
      name: this.name,
      size: this.tokens.size,
      depth: 1,
      entries: this.size,
      hitRate: this.queryCount > 0 ? this.hitCount / this.queryCount : 0,
    };
  }

  // ============================================================================
  // Private Methods
  // ============================================================================

  private extractText(data: DocumentData): string {
    const parts: string[] = [];

    for (const field of this.fields) {
      const value = this.getNestedValue(data, field);
      if (value !== undefined && value !== null) {
        if (Array.isArray(value)) {
          parts.push(value.map(String).join(' '));
        } else {
          parts.push(String(value));
        }
      }
    }

    return parts.join(' ');
  }

  private tokenize(text: string): string[] {
    const normalized = this.normalize(text);
    const words = normalized.split(/\s+/).filter((w) => w.length > 0);

    return words
      .filter((word) => !this.stopWords.has(word))
      .map((word) => this.stem(word));
  }

  private normalize(text: string): string {
    return text
      .toLowerCase()
      .replace(/[^\w\s]/g, ' ')
      .replace(/\s+/g, ' ')
      .trim();
  }

  private stem(word: string): string {
    // Basic Porter stemmer rules (simplified)
    if (word.length < 3) return word;

    // Remove common suffixes
    if (word.endsWith('ies') && word.length > 4) {
      return word.slice(0, -3) + 'y';
    }
    if (word.endsWith('es') && word.length > 3) {
      return word.slice(0, -2);
    }
    if (word.endsWith('s') && !word.endsWith('ss') && word.length > 3) {
      return word.slice(0, -1);
    }
    if (word.endsWith('ing') && word.length > 5) {
      return word.slice(0, -3);
    }
    if (word.endsWith('ed') && word.length > 4) {
      return word.slice(0, -2);
    }
    if (word.endsWith('ly') && word.length > 4) {
      return word.slice(0, -2);
    }

    return word;
  }

  private calculateScore(docId: DocumentId, token: string, frequency: number): number {
    const docLength = this.documentLengths.get(docId) || 1;
    const tokenInfo = this.tokens.get(token);
    const docFreq = tokenInfo ? tokenInfo.documents.size : 1;
    const totalDocs = this.documentLengths.size || 1;

    // TF-IDF scoring
    const tf = frequency / docLength;
    const idf = Math.log(totalDocs / docFreq);

    return tf * idf;
  }

  private levenshteinDistance(a: string, b: string): number {
    if (a.length === 0) return b.length;
    if (b.length === 0) return a.length;

    const matrix: number[][] = [];

    for (let i = 0; i <= b.length; i++) {
      matrix[i] = [i];
    }

    for (let j = 0; j <= a.length; j++) {
      matrix[0][j] = j;
    }

    for (let i = 1; i <= b.length; i++) {
      for (let j = 1; j <= a.length; j++) {
        const cost = a[j - 1] === b[i - 1] ? 0 : 1;
        matrix[i][j] = Math.min(
          matrix[i - 1][j] + 1,
          matrix[i][j - 1] + 1,
          matrix[i - 1][j - 1] + cost
        );
      }
    }

    return matrix[b.length][a.length];
  }

  private getNestedValue(obj: any, path: string): any {
    const parts = path.split('.');
    let current = obj;

    for (const part of parts) {
      if (current === null || current === undefined) {
        return undefined;
      }
      current = current[part];
    }

    return current;
  }
}
