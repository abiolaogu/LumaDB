
package io.lumadb.trino;

import io.trino.spi.connector.*;

public class LumaTransactionHandle implements ConnectorTransactionHandle {}

public class LumaTableHandle implements ConnectorTableHandle {
    private final SchemaTableName tableName;
    public LumaTableHandle(SchemaTableName tableName) { this.tableName = tableName; }
}

public class LumaSplitManager implements ConnectorSplitManager {
    @Override
    public ConnectorSplitSource getSplits(
            ConnectorTransactionHandle transaction,
            ConnectorSession session,
            ConnectorTableHandle table,
            DynamicFilter dynamicFilter,
            Constraint constraint) {
        return new FixedSplitSource(List.of());
    }
}

public class LumaRecordSetProvider implements ConnectorRecordSetProvider {
    @Override
    public RecordSet getRecordSet(
            ConnectorTransactionHandle transaction,
            ConnectorSession session,
            ConnectorSplit split,
            io.trino.spi.connector.ConnectorTableHandle table,
            java.util.List<? extends ColumnHandle> columns) {
        // Return record set that talks to LumaDB (via JDBC or gRPC)
        return null;
    }
}
