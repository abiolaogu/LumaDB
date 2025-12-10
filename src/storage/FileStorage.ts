/**
 * TDB+ File Storage Engine
 *
 * Persistent file-based storage with WAL (Write-Ahead Logging)
 * for durability and crash recovery.
 */

import * as fs from 'fs';
import * as path from 'path';
import { StorageEngine } from './StorageEngine';
import { StorageOptions, StoredDocument, StorageStats, CollectionName } from '../types';

interface WALEntry {
  timestamp: number;
  operation: 'INSERT' | 'UPDATE' | 'DELETE';
  collection: string;
  documentId: string;
  data?: any;
}

export class FileStorage extends StorageEngine {
  private basePath: string;
  private collections: Map<CollectionName, Map<string, StoredDocument>>;
  private walBuffer: WALEntry[];
  private walFlushInterval: NodeJS.Timeout | null;
  private cacheSize: number;

  constructor(options: StorageOptions) {
    super(options);
    this.basePath = options.path || './tdb_data';
    this.collections = new Map();
    this.walBuffer = [];
    this.walFlushInterval = null;
    this.cacheSize = options.cacheSize || 1000;
  }

  async initialize(): Promise<void> {
    // Ensure base directory exists
    await this.ensureDirectory(this.basePath);
    await this.ensureDirectory(path.join(this.basePath, 'collections'));
    await this.ensureDirectory(path.join(this.basePath, 'wal'));

    // Recover from WAL if needed
    await this.recoverFromWAL();

    // Load existing collections
    await this.loadAllCollections();

    // Start WAL flush interval (every 100ms)
    this.walFlushInterval = setInterval(() => {
      this.flushWAL().catch(console.error);
    }, 100);

    console.log(`File storage initialized at ${this.basePath}`);
  }

  async close(): Promise<void> {
    // Stop WAL flush interval
    if (this.walFlushInterval) {
      clearInterval(this.walFlushInterval);
      this.walFlushInterval = null;
    }

    // Flush any remaining WAL entries
    await this.flushWAL();

    // Save all collections
    for (const name of this.collections.keys()) {
      await this.flushCollection(name);
    }

    this.collections.clear();
    console.log('File storage closed');
  }

  async listCollections(): Promise<CollectionName[]> {
    const collectionsPath = path.join(this.basePath, 'collections');

    try {
      const files = await fs.promises.readdir(collectionsPath);
      return files
        .filter((f) => f.endsWith('.json'))
        .map((f) => f.replace('.json', ''));
    } catch {
      return [];
    }
  }

  async loadCollection(name: CollectionName): Promise<StoredDocument[]> {
    // Check cache first
    if (this.collections.has(name)) {
      return Array.from(this.collections.get(name)!.values());
    }

    const filePath = path.join(this.basePath, 'collections', `${name}.json`);

    try {
      const content = await fs.promises.readFile(filePath, 'utf-8');
      const documents: StoredDocument[] = JSON.parse(content);

      // Restore dates
      const restored = documents.map((doc) => ({
        ...doc,
        _createdAt: new Date(doc._createdAt),
        _updatedAt: new Date(doc._updatedAt),
      }));

      // Cache the collection
      const cache = new Map<string, StoredDocument>();
      for (const doc of restored) {
        cache.set(doc._id, doc);
      }
      this.collections.set(name, cache);

      return restored;
    } catch (error: any) {
      if (error.code === 'ENOENT') {
        // Collection doesn't exist yet
        this.collections.set(name, new Map());
        return [];
      }
      throw error;
    }
  }

  async saveDocument(collection: CollectionName, document: StoredDocument): Promise<void> {
    // Ensure collection cache exists
    if (!this.collections.has(collection)) {
      this.collections.set(collection, new Map());
    }

    // Update cache
    this.collections.get(collection)!.set(document._id, { ...document });

    // Write to WAL
    this.walBuffer.push({
      timestamp: Date.now(),
      operation: 'INSERT',
      collection,
      documentId: document._id,
      data: document,
    });
  }

  async deleteDocument(collection: CollectionName, documentId: string): Promise<void> {
    // Update cache
    const col = this.collections.get(collection);
    if (col) {
      col.delete(documentId);
    }

    // Write to WAL
    this.walBuffer.push({
      timestamp: Date.now(),
      operation: 'DELETE',
      collection,
      documentId,
    });
  }

  async dropCollection(name: CollectionName): Promise<void> {
    // Remove from cache
    this.collections.delete(name);

    // Remove file
    const filePath = path.join(this.basePath, 'collections', `${name}.json`);
    try {
      await fs.promises.unlink(filePath);
    } catch (error: any) {
      if (error.code !== 'ENOENT') {
        throw error;
      }
    }
  }

  async flushCollection(name: CollectionName): Promise<void> {
    const col = this.collections.get(name);
    if (!col) return;

    const documents = Array.from(col.values());
    const filePath = path.join(this.basePath, 'collections', `${name}.json`);

    await fs.promises.writeFile(filePath, JSON.stringify(documents, null, 2));
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

  // ============================================================================
  // Private Methods
  // ============================================================================

  private async ensureDirectory(dirPath: string): Promise<void> {
    try {
      await fs.promises.mkdir(dirPath, { recursive: true });
    } catch (error: any) {
      if (error.code !== 'EEXIST') {
        throw error;
      }
    }
  }

  private async loadAllCollections(): Promise<void> {
    const collectionNames = await this.listCollections();
    for (const name of collectionNames) {
      await this.loadCollection(name);
    }
  }

  private async flushWAL(): Promise<void> {
    if (this.walBuffer.length === 0) return;

    const entries = this.walBuffer;
    this.walBuffer = [];

    // Group by collection and flush
    const byCollection = new Map<string, WALEntry[]>();
    for (const entry of entries) {
      if (!byCollection.has(entry.collection)) {
        byCollection.set(entry.collection, []);
      }
      byCollection.get(entry.collection)!.push(entry);
    }

    // Flush each affected collection
    for (const collection of byCollection.keys()) {
      await this.flushCollection(collection);
    }
  }

  private async recoverFromWAL(): Promise<void> {
    const walPath = path.join(this.basePath, 'wal');

    try {
      const files = await fs.promises.readdir(walPath);
      const walFiles = files.filter((f) => f.endsWith('.wal')).sort();

      for (const walFile of walFiles) {
        const filePath = path.join(walPath, walFile);
        const content = await fs.promises.readFile(filePath, 'utf-8');
        const entries: WALEntry[] = content
          .split('\n')
          .filter((line) => line.trim())
          .map((line) => JSON.parse(line));

        // Apply each entry
        for (const entry of entries) {
          await this.applyWALEntry(entry);
        }

        // Remove processed WAL file
        await fs.promises.unlink(filePath);
      }

      // Flush all recovered data
      for (const name of this.collections.keys()) {
        await this.flushCollection(name);
      }
    } catch (error: any) {
      if (error.code !== 'ENOENT') {
        console.error('Error recovering from WAL:', error);
      }
    }
  }

  private async applyWALEntry(entry: WALEntry): Promise<void> {
    if (!this.collections.has(entry.collection)) {
      this.collections.set(entry.collection, new Map());
    }

    const col = this.collections.get(entry.collection)!;

    switch (entry.operation) {
      case 'INSERT':
      case 'UPDATE':
        if (entry.data) {
          col.set(entry.documentId, entry.data);
        }
        break;
      case 'DELETE':
        col.delete(entry.documentId);
        break;
    }
  }
}
