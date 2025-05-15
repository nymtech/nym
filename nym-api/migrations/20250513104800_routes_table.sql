-- Drop the table if it exists
DROP TABLE IF EXISTS routes;

-- Create the routes table
CREATE TABLE routes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  layer1 INTEGER,
  layer2 INTEGER,
  layer3 INTEGER,
  gw INTEGER,
  success BOOLEAN
);

-- Create an index on created_at
CREATE INDEX routes_created_at ON routes(created_at);
