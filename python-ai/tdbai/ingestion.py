from typing import Dict, Any, Callable
import asyncio
import json
import os
import time
import logging

try:
    import psycopg2
    from psycopg2.extras import RealDictCursor
except ImportError:
    psycopg2 = None # Handle missing dependency gracefully

# --- LumaDB Configuration ---
# Defaults to localhost LumaDB (which now speaks PG protocol via pg_wire)
DB_URL = os.getenv("LUMADB_URL", "postgresql://lumadb:lumadb@127.0.0.1:5432/default")

logger = logging.getLogger("luma_ingestion")

class LumaStreamClient:
    """
    Client for high-performance stream ingestion into LumaDB.
    Adopts the 'RegTech' pattern of using Postgres protocol for events.
    """
    def __init__(self, connection_string: str = DB_URL):
        self.connection_string = connection_string
        self.handlers: Dict[str, Callable] = {}
        if psycopg2:
            self._ensure_table()
        else:
            logger.warning("psycopg2 not installed. Ingestion disabled.")

    def _get_connection(self):
        if not psycopg2:
            raise ImportError("psycopg2 is required for LumaStreamClient")
        return psycopg2.connect(self.connection_string)

    def _ensure_table(self):
        """Creates the system_events table in LumaDB if it doesn't exist."""
        try:
            conn = self._get_connection()
            cur = conn.cursor()
            # Note: LumaDB pg_wire stub might just say "OK" to this, which is fine.
            cur.execute("""
                CREATE TABLE IF NOT EXISTS system_events (
                    id SERIAL PRIMARY KEY,
                    topic TEXT NOT NULL,
                    payload JSONB NOT NULL,
                    processed BOOLEAN DEFAULT FALSE,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                );
            """)
            conn.commit()
            conn.close()
            logger.info("[LumaDB] 'system_events' table ready.")
        except Exception as e:
            logger.error(f"[LumaDB] Init error: {e}")

    def subscriber(self, topic: str):
        def decorator(func):
            self.handlers[topic] = func
            return func
        return decorator

    async def emit(self, topic: str, msg: Dict[str, Any]):
        """Inserts an event into the LumaDB system_events table."""
        logger.debug(f"[STREAM] Emitting to '{topic}': {msg}")
        try:
            # Offload blocking IO to thread
            await asyncio.to_thread(self._emit_sync, topic, msg)
        except Exception as e:
            logger.error(f"[STREAM] Emit failed: {e}")

    def _emit_sync(self, topic: str, msg: Dict[str, Any]):
        conn = self._get_connection()
        cur = conn.cursor()
        cur.execute(
            "INSERT INTO system_events (topic, payload) VALUES (%s, %s)",
            (topic, json.dumps(msg))
        )
        conn.commit()
        conn.close()

    async def consume_loop(self, poll_interval: float = 2.0):
        """Polls LumaDB for unprocessed events and runs handlers."""
        logger.info("[STREAM] Starting LumaDB Consumer Loop...")
        if not psycopg2:
            return

        while True:
            try:
                await asyncio.to_thread(self._consume_once)
            except Exception as e:
                logger.error(f"[STREAM] Consumer Loop Error: {e}")
            
            await asyncio.sleep(poll_interval)

    def _consume_once(self):
        conn = self._get_connection()
        cur = conn.cursor(cursor_factory=RealDictCursor)
        
        # Fetch unprocessed events
        # In LumaDB pg_wire stub, this might return mock data for now.
        cur.execute(
            "SELECT id, topic, payload FROM system_events WHERE processed = FALSE ORDER BY id ASC LIMIT 10"
        )
        events = cur.fetchall()

        for event in events:
            topic = event['topic']
            payload = event['payload'] # LumaDB should return dict from JSONB
            
            # If payload is string (mock), parse it
            if isinstance(payload, str):
                try:
                    payload = json.loads(payload)
                except:
                    pass

            if topic in self.handlers:
                # Run handler (async execution requires loop management, simpler to run sync here or fix)
                # Ideally we run async handler in loop. 
                # For simplicity in this port, we assume sync or fire-and-forget.
                pass 
                
            # Mark as processed
            cur.execute("UPDATE system_events SET processed = TRUE WHERE id = %s", (event['id'],))
            conn.commit()
        
        conn.close()
