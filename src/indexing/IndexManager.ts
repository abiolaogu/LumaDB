/**
 * TDB+ Index Manager
 *
 * Manages all indexes across the database, providing unified
 * access to different index types.
 */

import { BTreeIndex } from './BTreeIndex';
import { HashIndex } from './HashIndex';
import { FullTextIndex } from './FullTextIndex';
import {
  IndexDefinition,
  IndexType,
  IndexStats,
  DocumentId,
  DocumentData,
  ComparisonOperator,
  IndexError,
} from '../types';

type IndexInstance = BTreeIndex | HashIndex | FullTextIndex;

export class IndexManager {
  private indexes: Map<string, Map<string, IndexInstance>>; // collection -> indexName -> index
  private definitions: Map<string, Map<string, IndexDefinition>>; // collection -> indexName -> definition

  constructor() {
    this.indexes = new Map();
    this.definitions = new Map();
  }

  /**
   * Create a new index
   */
  async createIndex(definition: IndexDefinition): Promise<void> {
    const { collection, name, type, fields, unique, options } = definition;

    // Ensure collection maps exist
    if (!this.indexes.has(collection)) {
      this.indexes.set(collection, new Map());
      this.definitions.set(collection, new Map());
    }

    // Check if index already exists
    if (this.indexes.get(collection)!.has(name)) {
      throw new IndexError(`Index ${name} already exists on collection ${collection}`, name);
    }

    // Create the appropriate index type
    let index: IndexInstance;

    switch (type) {
      case 'btree':
        index = new BTreeIndex(name, fields, unique, options?.order);
        break;
      case 'hash':
        index = new HashIndex(name, fields, unique);
        break;
      case 'fulltext':
        index = new FullTextIndex(name, fields, options?.language, options?.stopWords);
        break;
      default:
        throw new IndexError(`Unknown index type: ${type}`, name);
    }

    this.indexes.get(collection)!.set(name, index);
    this.definitions.get(collection)!.set(name, definition);

    console.log(`Created ${type} index "${name}" on ${collection}(${fields.join(', ')})`);
  }

  /**
   * Drop an index
   */
  async dropIndex(collection: string, name: string): Promise<boolean> {
    const collectionIndexes = this.indexes.get(collection);
    if (!collectionIndexes || !collectionIndexes.has(name)) {
      return false;
    }

    collectionIndexes.delete(name);
    this.definitions.get(collection)?.delete(name);

    console.log(`Dropped index "${name}" from ${collection}`);
    return true;
  }

  /**
   * Index a document
   */
  indexDocument(collection: string, documentId: DocumentId, data: DocumentData): void {
    const collectionIndexes = this.indexes.get(collection);
    if (!collectionIndexes) return;

    for (const index of collectionIndexes.values()) {
      index.insert(documentId, data);
    }
  }

  /**
   * Remove a document from indexes
   */
  unindexDocument(collection: string, documentId: DocumentId, data: DocumentData): void {
    const collectionIndexes = this.indexes.get(collection);
    if (!collectionIndexes) return;

    for (const index of collectionIndexes.values()) {
      index.remove(documentId, data);
    }
  }

  /**
   * Query an index
   */
  query(
    collection: string,
    indexName: string,
    operator: ComparisonOperator,
    value: any
  ): DocumentId[] {
    const index = this.indexes.get(collection)?.get(indexName);
    if (!index) {
      return [];
    }

    return index.query(operator, value);
  }

  /**
   * Find an index that can be used for a field
   */
  findIndexForField(collection: string, field: string): IndexDefinition | null {
    const collectionDefs = this.definitions.get(collection);
    if (!collectionDefs) return null;

    for (const definition of collectionDefs.values()) {
      if (definition.fields.includes(field) || definition.fields[0] === field) {
        return definition;
      }
    }

    return null;
  }

  /**
   * Get all indexes for a collection
   */
  getIndexesForCollection(collection: string): IndexDefinition[] {
    const collectionDefs = this.definitions.get(collection);
    if (!collectionDefs) return [];
    return Array.from(collectionDefs.values());
  }

  /**
   * Get index statistics
   */
  getIndexStats(collection: string, indexName: string): IndexStats | null {
    const index = this.indexes.get(collection)?.get(indexName);
    if (!index) return null;

    return index.getStats();
  }

  /**
   * Get all index statistics for a collection
   */
  getAllIndexStats(collection: string): IndexStats[] {
    const collectionIndexes = this.indexes.get(collection);
    if (!collectionIndexes) return [];

    return Array.from(collectionIndexes.values()).map((index) => index.getStats());
  }

  /**
   * Rebuild an index
   */
  async rebuildIndex(
    collection: string,
    indexName: string,
    documents: Map<DocumentId, DocumentData>
  ): Promise<void> {
    const index = this.indexes.get(collection)?.get(indexName);
    if (!index) {
      throw new IndexError(`Index ${indexName} not found`, indexName);
    }

    index.clear();

    for (const [id, data] of documents) {
      index.insert(id, data);
    }

    console.log(`Rebuilt index "${indexName}" with ${documents.size} documents`);
  }
}
