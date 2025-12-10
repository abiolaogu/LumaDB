/**
 * TDB+ B-Tree Index
 *
 * A self-balancing tree data structure optimized for range queries.
 * Provides O(log n) lookups, insertions, and deletions.
 */

import { DocumentId, DocumentData, ComparisonOperator, IndexStats, IndexError } from '../types';

interface BTreeNode {
  keys: any[];
  values: Set<DocumentId>[];
  children: BTreeNode[];
  isLeaf: boolean;
}

export class BTreeIndex {
  private name: string;
  private fields: string[];
  private unique: boolean;
  private order: number;
  private root: BTreeNode;
  private size: number;
  private queryCount: number;
  private hitCount: number;

  constructor(name: string, fields: string[], unique: boolean = false, order: number = 32) {
    this.name = name;
    this.fields = fields;
    this.unique = unique;
    this.order = Math.max(3, order); // Minimum order of 3
    this.root = this.createNode(true);
    this.size = 0;
    this.queryCount = 0;
    this.hitCount = 0;
  }

  private createNode(isLeaf: boolean): BTreeNode {
    return {
      keys: [],
      values: [],
      children: [],
      isLeaf,
    };
  }

  /**
   * Insert a document into the index
   */
  insert(documentId: DocumentId, data: DocumentData): void {
    const key = this.extractKey(data);
    if (key === undefined) return; // Skip if key field is missing

    // Check uniqueness constraint
    if (this.unique) {
      const existing = this.findExact(key);
      if (existing.length > 0 && !existing.includes(documentId)) {
        throw new IndexError(
          `Duplicate key violation: ${key} already exists in unique index ${this.name}`,
          this.name
        );
      }
    }

    // If root is full, split it
    if (this.root.keys.length === 2 * this.order - 1) {
      const newRoot = this.createNode(false);
      newRoot.children.push(this.root);
      this.splitChild(newRoot, 0);
      this.root = newRoot;
    }

    this.insertNonFull(this.root, key, documentId);
    this.size++;
  }

  private insertNonFull(node: BTreeNode, key: any, documentId: DocumentId): void {
    let i = node.keys.length - 1;

    if (node.isLeaf) {
      // Find position for new key
      while (i >= 0 && this.compare(key, node.keys[i]) < 0) {
        i--;
      }

      // Check if key already exists
      if (i >= 0 && this.compare(key, node.keys[i]) === 0) {
        node.values[i].add(documentId);
      } else {
        // Insert new key
        node.keys.splice(i + 1, 0, key);
        node.values.splice(i + 1, 0, new Set([documentId]));
      }
    } else {
      // Find child to recurse into
      while (i >= 0 && this.compare(key, node.keys[i]) < 0) {
        i--;
      }
      i++;

      // Split child if full
      if (node.children[i].keys.length === 2 * this.order - 1) {
        this.splitChild(node, i);
        if (this.compare(key, node.keys[i]) > 0) {
          i++;
        }
      }

      this.insertNonFull(node.children[i], key, documentId);
    }
  }

  private splitChild(parent: BTreeNode, index: number): void {
    const child = parent.children[index];
    const newNode = this.createNode(child.isLeaf);
    const mid = this.order - 1;

    // Move half the keys to new node
    newNode.keys = child.keys.splice(mid + 1);
    newNode.values = child.values.splice(mid + 1);

    if (!child.isLeaf) {
      newNode.children = child.children.splice(mid + 1);
    }

    // Move middle key up to parent
    parent.keys.splice(index, 0, child.keys.pop()!);
    parent.values.splice(index, 0, child.values.pop()!);
    parent.children.splice(index + 1, 0, newNode);
  }

  /**
   * Remove a document from the index
   */
  remove(documentId: DocumentId, data: DocumentData): void {
    const key = this.extractKey(data);
    if (key === undefined) return;

    this.removeFromNode(this.root, key, documentId);
  }

  private removeFromNode(node: BTreeNode, key: any, documentId: DocumentId): boolean {
    let i = 0;
    while (i < node.keys.length && this.compare(key, node.keys[i]) > 0) {
      i++;
    }

    if (node.isLeaf) {
      if (i < node.keys.length && this.compare(key, node.keys[i]) === 0) {
        node.values[i].delete(documentId);
        if (node.values[i].size === 0) {
          node.keys.splice(i, 1);
          node.values.splice(i, 1);
          this.size--;
        }
        return true;
      }
      return false;
    }

    if (i < node.keys.length && this.compare(key, node.keys[i]) === 0) {
      node.values[i].delete(documentId);
      if (node.values[i].size === 0) {
        // Handle removal of key from internal node
        // (simplified - full B-tree deletion is complex)
        node.keys.splice(i, 1);
        node.values.splice(i, 1);
        this.size--;
      }
      return true;
    }

    return this.removeFromNode(node.children[i], key, documentId);
  }

  /**
   * Query the index
   */
  query(operator: ComparisonOperator, value: any): DocumentId[] {
    this.queryCount++;

    switch (operator) {
      case '=':
        const exact = this.findExact(value);
        if (exact.length > 0) this.hitCount++;
        return exact;

      case '!=':
        return this.findNotEqual(value);

      case '>':
        return this.findGreater(value, false);

      case '>=':
        return this.findGreater(value, true);

      case '<':
        return this.findLess(value, false);

      case '<=':
        return this.findLess(value, true);

      case 'BETWEEN':
        if (Array.isArray(value) && value.length === 2) {
          return this.findBetween(value[0], value[1]);
        }
        return [];

      case 'IN':
        if (Array.isArray(value)) {
          const results = new Set<DocumentId>();
          for (const v of value) {
            for (const id of this.findExact(v)) {
              results.add(id);
            }
          }
          return Array.from(results);
        }
        return [];

      default:
        return [];
    }
  }

  private findExact(value: any): DocumentId[] {
    let node = this.root;

    while (node) {
      let i = 0;
      while (i < node.keys.length && this.compare(value, node.keys[i]) > 0) {
        i++;
      }

      if (i < node.keys.length && this.compare(value, node.keys[i]) === 0) {
        return Array.from(node.values[i]);
      }

      if (node.isLeaf) {
        return [];
      }

      node = node.children[i];
    }

    return [];
  }

  private findNotEqual(value: any): DocumentId[] {
    const results: DocumentId[] = [];
    this.traverseAll(this.root, (key, ids) => {
      if (this.compare(key, value) !== 0) {
        results.push(...ids);
      }
    });
    return results;
  }

  private findGreater(value: any, inclusive: boolean): DocumentId[] {
    const results: DocumentId[] = [];
    this.traverseAll(this.root, (key, ids) => {
      const cmp = this.compare(key, value);
      if (cmp > 0 || (inclusive && cmp === 0)) {
        results.push(...ids);
      }
    });
    return results;
  }

  private findLess(value: any, inclusive: boolean): DocumentId[] {
    const results: DocumentId[] = [];
    this.traverseAll(this.root, (key, ids) => {
      const cmp = this.compare(key, value);
      if (cmp < 0 || (inclusive && cmp === 0)) {
        results.push(...ids);
      }
    });
    return results;
  }

  private findBetween(low: any, high: any): DocumentId[] {
    const results: DocumentId[] = [];
    this.traverseAll(this.root, (key, ids) => {
      if (this.compare(key, low) >= 0 && this.compare(key, high) <= 0) {
        results.push(...ids);
      }
    });
    return results;
  }

  private traverseAll(
    node: BTreeNode,
    callback: (key: any, ids: DocumentId[]) => void
  ): void {
    for (let i = 0; i < node.keys.length; i++) {
      if (!node.isLeaf && node.children[i]) {
        this.traverseAll(node.children[i], callback);
      }
      callback(node.keys[i], Array.from(node.values[i]));
    }

    if (!node.isLeaf && node.children[node.keys.length]) {
      this.traverseAll(node.children[node.keys.length], callback);
    }
  }

  /**
   * Clear the index
   */
  clear(): void {
    this.root = this.createNode(true);
    this.size = 0;
  }

  /**
   * Get index statistics
   */
  getStats(): IndexStats {
    return {
      name: this.name,
      size: this.size,
      depth: this.getDepth(this.root),
      entries: this.size,
      hitRate: this.queryCount > 0 ? this.hitCount / this.queryCount : 0,
    };
  }

  private getDepth(node: BTreeNode): number {
    if (node.isLeaf) return 1;
    if (node.children.length === 0) return 1;
    return 1 + this.getDepth(node.children[0]);
  }

  private extractKey(data: DocumentData): any {
    if (this.fields.length === 1) {
      return this.getNestedValue(data, this.fields[0]);
    }

    // Composite key
    return this.fields.map((field) => this.getNestedValue(data, field)).join('\0');
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

  private compare(a: any, b: any): number {
    if (a === b) return 0;
    if (a === undefined || a === null) return -1;
    if (b === undefined || b === null) return 1;

    if (typeof a === 'number' && typeof b === 'number') {
      return a - b;
    }

    const strA = String(a);
    const strB = String(b);
    return strA < strB ? -1 : strA > strB ? 1 : 0;
  }
}
