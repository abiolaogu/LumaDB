
package io.lumadb.trino;

import io.trino.spi.connector.Connector;
import io.trino.spi.connector.ConnectorContext;
import io.trino.spi.connector.ConnectorFactory;
import java.util.Map;

public class LumaConnectorFactory implements ConnectorFactory {
    @Override
    public String getName() {
        return "luma";
    }

    @Override
    public Connector create(String catalogName, Map<String, String> config, ConnectorContext context) {
        return new LumaConnector(); // Real impl implies dependency injection here
    }
}
