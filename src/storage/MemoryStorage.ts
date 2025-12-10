/**
 * TDB+ Memory Storage Engine
 *
 * In-memory storage for development and high-performance scenarios.
 * All data is lost when the process exits.
 */

import { StorageEngine } from './StorageEngine';
import { StorageOptions, StoredDocument, StorageStats, CollectionName } from '../types';

export class MemoryStorage extends StorageEngine {
  private collections: Map<CollectionName, Map<string, StoredDocument>>;

  constructor(options: StorageOptions) {
    super(options);
    this.collections = new Map();
  }

  async initialize(): Promise<void> {
    // Memory storage requires no initialization
    console.log('Memory storage initialized');
  }

  async close(): Promise<void> {
    // Clear all data
    this.collections.clear();
    console.log('Memory storage closed');
  }

  async listCollections(): Promise<CollectionName[]> {
    return Array.from(this.collections.keys());
  }

  async loadCollection(name: CollectionName): Promise<StoredDocument[]> {
    const collection = this.collections.get(name);
    if (!collection) {
      return [];
    }
    return Array.from(collection.values());
  }

  async saveDocument(collection: CollectionName, document: StoredDocument): Promise<void> {
    if (!this.collections.has(collection)) {
      this.collections.set(collection, new Map());
    }

    this.collections.get(collection)!.set(document._id, {
      ...document,
      _createdAt: new Date(document._createdAt),
      _updatedAt: new Date(document._updatedAt),
    });
  }

  async deleteDocument(collection: CollectionName, documentId: string): Promise<void> {
    const col = this.collections.get(collection);
    if (col) {
      col.delete(documentId);
    }
  }

  async dropCollection(name: CollectionName): Promise<void> {
    this.collections.delete(name);
  }

  async flushCollection(_name: CollectionName): Promise<void> {
    // No-op for memory storage
  }

  async getStats(): Promise<StorageStats> {
    let totalDocuments = 0;
    let totalSize = 0;
    const collectionStats = [];

    for (const [name, docs] of this.collections) {
      const documents = Array.from(docs.values());
      const size = documents.reduce(
        (sum, doc) => sum + JSON.stringify(doc).length,
        0
      );

      totalDocuments += documents.length;
      totalSize += size;

      collectionStats.push({
        name,
        documentCount: documents.length,
        size,
        indexes: [],
        avgDocumentSize: documents.length > 0 ? size / documents.length : 0,
      });
    }

    return {
      totalDocuments,
      totalSize,
      collections: collectionStats,
    };
  }
}
