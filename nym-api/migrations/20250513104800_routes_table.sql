-- Add routes table for storing route metrics data
CREATE TABLE IF NOT EXISTS routes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    layer1 INTEGER NOT NULL,   -- NodeId of layer 1 mixnode
    layer2 INTEGER NOT NULL,   -- NodeId of layer 2 mixnode
    layer3 INTEGER NOT NULL,   -- NodeId of layer 3 mixnode
    gw INTEGER NOT NULL,       -- NodeId of gateway
    success BOOLEAN NOT NULL,  -- Whether the packet was delivered successfully
    timestamp INTEGER NOT NULL DEFAULT (unixepoch()) -- When the measurement was taken
);

-- Add indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_routes_timestamp ON routes(timestamp);
CREATE INDEX IF NOT EXISTS idx_routes_layer1 ON routes(layer1);
CREATE INDEX IF NOT EXISTS idx_routes_layer2 ON routes(layer2);
CREATE INDEX IF NOT EXISTS idx_routes_layer3 ON routes(layer3);
CREATE INDEX IF NOT EXISTS idx_routes_gw ON routes(gw);