
package io.lumadb.trino;

import io.trino.spi.Plugin;
import io.trino.spi.connector.ConnectorFactory;
import com.google.common.collect.ImmutableList;

public class LumaPlugin implements Plugin {
    @Override
    public Iterable<ConnectorFactory> getConnectorFactories() {
        return ImmutableList.of(new LumaConnectorFactory());
    }
}
