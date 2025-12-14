
package io.lumadb.trino;

import io.trino.spi.connector.*;
import java.util.List;
import java.util.Map;
import java.util.Optional;

public class LumaMetadata implements ConnectorMetadata {
    @Override
    public List<String> listSchemaNames(ConnectorSession session) {
        return List.of("default", "system");
    }

    @Override
    public ConnectorTableHandle getTableHandle(ConnectorSession session, SchemaTableName tableName) {
        return new LumaTableHandle(tableName);
    }

    @Override
    public ConnectorTableMetadata getTableMetadata(ConnectorSession session, ConnectorTableHandle table) {
        // Stub: In real impl, query LumaDB gRPC Metadata API
        return null; 
    }

    @Override
    public List<SchemaTableName> listTables(ConnectorSession session, Optional<String> schemaName) {
        return List.of();
    }
}
