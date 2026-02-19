-- Magda test keyspace and sample data

CREATE KEYSPACE IF NOT EXISTS magda_test
WITH replication = {'class': 'SimpleStrategy', 'replication_factor': 1};

USE magda_test;

-- Users table
CREATE TABLE IF NOT EXISTS users (
    id UUID PRIMARY KEY,
    name TEXT,
    email TEXT,
    age INT,
    active BOOLEAN,
    created_at TIMESTAMP
);

-- Products table
CREATE TABLE IF NOT EXISTS products (
    id UUID PRIMARY KEY,
    name TEXT,
    category TEXT,
    price DECIMAL,
    stock INT,
    tags SET<TEXT>,
    created_at TIMESTAMP
);

-- Orders table (with clustering key)
CREATE TABLE IF NOT EXISTS orders (
    user_id UUID,
    order_id TIMEUUID,
    product_id UUID,
    quantity INT,
    total DECIMAL,
    status TEXT,
    PRIMARY KEY (user_id, order_id)
) WITH CLUSTERING ORDER BY (order_id DESC);

-- Insert sample users
INSERT INTO users (id, name, email, age, active, created_at)
VALUES (uuid(), 'Alice Silva', 'alice@example.com', 30, true, toTimestamp(now()));

INSERT INTO users (id, name, email, age, active, created_at)
VALUES (uuid(), 'Bob Santos', 'bob@example.com', 25, true, toTimestamp(now()));

INSERT INTO users (id, name, email, age, active, created_at)
VALUES (uuid(), 'Carol Lima', 'carol@example.com', 35, false, toTimestamp(now()));

INSERT INTO users (id, name, email, age, active, created_at)
VALUES (uuid(), 'David Costa', 'david@example.com', 28, true, toTimestamp(now()));

INSERT INTO users (id, name, email, age, active, created_at)
VALUES (uuid(), 'Eva Oliveira', 'eva@example.com', 42, true, toTimestamp(now()));

-- Insert sample products
INSERT INTO products (id, name, category, price, stock, tags, created_at)
VALUES (uuid(), 'Laptop Pro', 'electronics', 2999.99, 50, {'tech', 'portable'}, toTimestamp(now()));

INSERT INTO products (id, name, category, price, stock, tags, created_at)
VALUES (uuid(), 'Wireless Mouse', 'electronics', 49.99, 200, {'tech', 'accessory'}, toTimestamp(now()));

INSERT INTO products (id, name, category, price, stock, tags, created_at)
VALUES (uuid(), 'Coffee Mug', 'kitchen', 15.00, 500, {'home', 'drink'}, toTimestamp(now()));

INSERT INTO products (id, name, category, price, stock, tags, created_at)
VALUES (uuid(), 'Standing Desk', 'furniture', 899.00, 30, {'office', 'ergonomic'}, toTimestamp(now()));

INSERT INTO products (id, name, category, price, stock, tags, created_at)
VALUES (uuid(), 'Mechanical Keyboard', 'electronics', 149.99, 100, {'tech', 'accessory', 'gaming'}, toTimestamp(now()));
