/**
 * TDB+ Hash Index
 *
 * A hash-based index optimized for exact equality lookups.
 * Provides O(1) average case lookups, but doesn't support range queries.
 */

import { DocumentId, DocumentData, ComparisonOperator, IndexStats, IndexError } from '../types';

export class HashIndex {
  private name: string;
  private fields: string[];
  private unique: boolean;
  private buckets: Map<string, Set<DocumentId>>;
  private size: number;
  private queryCount: number;
  private hitCount: number;

  constructor(name: string, fields: string[], unique: boolean = false) {
    this.name = name;
    this.fields = fields;
    this.unique = unique;
    this.buckets = new Map();
    this.size = 0;
    this.queryCount = 0;
    this.hitCount = 0;
  }

  /**
   * Insert a document into the index
   */
  insert(documentId: DocumentId, data: DocumentData): void {
    const key = this.hashKey(data);
    if (key === undefined) return;

    // Check uniqueness constraint
    if (this.unique) {
      const existing = this.buckets.get(key);
      if (existing && existing.size > 0 && !existing.has(documentId)) {
        throw new IndexError(
          `Duplicate key violation: hash key already exists in unique index ${this.name}`,
          this.name
        );
      }
    }

    if (!this.buckets.has(key)) {
      this.buckets.set(key, new Set());
    }

    const bucket = this.buckets.get(key)!;
    if (!bucket.has(documentId)) {
      bucket.add(documentId);
      this.size++;
    }
  }

  /**
   * Remove a document from the index
   */
  remove(documentId: DocumentId, data: DocumentData): void {
    const key = this.hashKey(data);
    if (key === undefined) return;

    const bucket = this.buckets.get(key);
    if (bucket) {
      if (bucket.delete(documentId)) {
        this.size--;
      }
      if (bucket.size === 0) {
        this.buckets.delete(key);
      }
    }
  }

  /**
   * Query the index
   */
  query(operator: ComparisonOperator, value: any): DocumentId[] {
    this.queryCount++;

    switch (operator) {
      case '=':
        const key = this.hashValue(value);
        const bucket = this.buckets.get(key);
        if (bucket && bucket.size > 0) {
          this.hitCount++;
          return Array.from(bucket);
        }
        return [];

      case '!=':
        const excludeKey = this.hashValue(value);
        const results: DocumentId[] = [];
        for (const [k, b] of this.buckets) {
          if (k !== excludeKey) {
            results.push(...b);
          }
        }
        return results;

      case 'IN':
        if (Array.isArray(value)) {
          const inResults = new Set<DocumentId>();
          for (const v of value) {
            const inKey = this.hashValue(v);
            const inBucket = this.buckets.get(inKey);
            if (inBucket) {
              for (const id of inBucket) {
                inResults.add(id);
              }
            }
          }
          if (inResults.size > 0) this.hitCount++;
          return Array.from(inResults);
        }
        return [];

      case 'NOT IN':
        if (Array.isArray(value)) {
          const excludeKeys = new Set(value.map((v) => this.hashValue(v)));
          const notInResults: DocumentId[] = [];
          for (const [k, b] of this.buckets) {
            if (!excludeKeys.has(k)) {
              notInResults.push(...b);
            }
          }
          return notInResults;
        }
        return [];

      default:
        // Hash index doesn't support range queries
        // Return empty and let the query engine do a full scan
        return [];
    }
  }

  /**
   * Clear the index
   */
  clear(): void {
    this.buckets.clear();
    this.size = 0;
  }

  /**
   * Get index statistics
   */
  getStats(): IndexStats {
    // Calculate average bucket depth
    let totalDepth = 0;
    for (const bucket of this.buckets.values()) {
      totalDepth += bucket.size;
    }
    const avgDepth = this.buckets.size > 0 ? totalDepth / this.buckets.size : 0;

    return {
      name: this.name,
      size: this.buckets.size,
      depth: Math.ceil(avgDepth),
      entries: this.size,
      hitRate: this.queryCount > 0 ? this.hitCount / this.queryCount : 0,
    };
  }

  private hashKey(data: DocumentData): string | undefined {
    const values: any[] = [];

    for (const field of this.fields) {
      const value = this.getNestedValue(data, field);
      if (value === undefined) return undefined;
      values.push(value);
    }

    return this.hashValue(values.length === 1 ? values[0] : values);
  }

  private hashValue(value: any): string {
    if (value === null || value === undefined) {
      return '__null__';
    }

    if (Array.isArray(value)) {
      return value.map((v) => this.hashValue(v)).join('\x00');
    }

    if (typeof value === 'object') {
      return JSON.stringify(value);
    }

    return String(value);
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
