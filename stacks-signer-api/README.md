# Stacks Signer API

This is an API for managing signers and their associated keys in a SQLite database. The API provides a set of RESTful endpoints to add, delete, and fetch signers and keys.

## Prerequisites

- Rust
- SQLite

## Dev/Compilation setup

To make use of `sqlx` and verify the sql queries on your own, you should follow the following steps:

1. remove `sqlx-data.json`
2. install `sqlx-cli`
3. make sure you have sqlite installed
4. create a `.env` file in the `stacks-signer-api` root folder with the env variable `DATABASE_URL`
5. The url for sqlite is in the format `DATABASE_URL=sqlite:////$(pwd)/stacks-signer-api/dev-signer-api.sqlite`
6. generate the test db using `sqlx database create`
7. you need to manually add the tables that the queries are accessing for compile time checking, i.e. the `sbtc_signer` and `keys` table

```Sql
CREATE TABLE IF NOT EXISTS sbtc_signers (
        signer_id INTEGER NOT NULL,
        user_id INTEGER NOT NULL,
        status TEXT NOT NULL,

        PRIMARY KEY(signer_id, user_id)
    );

CREATE TABLE IF NOT EXISTS keys (
            key TEXT NOT NULL,
            signer_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
    
            PRIMARY KEY(key, signer_id, user_id),
            FOREIGN KEY(signer_id, user_id) REFERENCES sbtc_signers(signer_id, user_id)
        );
```

## API Overview

The API is built using the Rust programming language and is designed to interact with a SQLite database. The provided endpoints allow you to manage signers, their status, and the associated keys.

The following endpoints are available:

- `POST /v1/signers`: Add a signer to the database
- `DELETE /v1/signers`: Delete a signer from the database
- `GET /v1/signers`: Get a list of signers from the database, with optional filtering by status
- `POST /v1/keys`: Add a key to the database
- `DELETE /v1/keys`: Delete a key from the database
- `GET /v1/keys`: Get a list of keys from the database, with optional pagination

## Usage

To use the API, you need to send HTTP requests to the corresponding endpoints with the required JSON payloads. Follow these general steps:

1. Set up your Rust environment and SQLite database.
2. Build and run the API server.
3. Use an HTTP client (e.g., `curl`, Postman, or a web application) to send requests to the exposed endpoints.

### Examples

Here are examples for each of the supported endpoints:

#### Add a signer

**Request**

```
POST /v1/signers
Content-Type: application/json

{
  "signer_id": 1,
  "user_id": 1,
  "status": "Active"
}
```

**Success Response**

```
HTTP/1.1 201 CREATED
Content-Type: application/json

{
  "status": "added"
}
```

#### Delete a signer

**Request**

```
DELETE /v1/signers
Content-Type: application/json

{
  "signer_id": 1,
  "user_id": 1
}
```

**Success Response**

```
HTTP/1.1 200 OK
Content-Type: application/json

{
  "status": "deleted"
}
```

#### Get signers

**Request**

```
GET /v1/signers?status=Active
```

**Success Response**

```
HTTP/1.1 200 OK
Content-Type: application/json

[
  {
    "signer_id": 1,
    "user_id": 1,
    "status": "Active"
  },
  {
    "signer_id": 2,
    "user_id": 2,
    "status": "Active"
  }
]
```

#### Add a key

**Request**

```
POST /v1/keys
Content-Type: application/json

{
  "signer_id": 1,
  "user_id": 1,
  "key": "example_key"
}
```

**Success Response**

```
HTTP/1.1 201 CREATED
Content-Type: application/json

{
  "status": "added"
}
```

#### Delete a key

**Request**

```
DELETE /v1/keys
Content-Type: application/json

{
  "signer_id": 1,
  "user_id": 1,
  "key": "example_key"
}
```

**Success Response**

```
HTTP/1.1 200 OK
Content-Type: application/json

{
  "status": "deleted"
}
```

#### Get keys

**Request**

```
GET /v1/keys?signer_id=1&user_id=1&page=1&limit=10
```

**Success Response**

```
HTTP/1.1 200 OK
Content-Type: application/json

[
  "example_key1",
  "example_key2",
  "example_key3"
]
```

## Failure Cases

In case of incorrect input or errors, the API will return appropriate HTTP status codes and JSON error messages, such as:

- 400 Bad Request: The request is malformed or missing required fields.
- 404 Not Found: The requested signer or key is not found in the database.
- 500 Internal Server Error: An unexpected error occurred on the server-side.

In each case, the API will return a JSON object with an `error` field describing the issue:

```
{
  "error": "Error message"
}
```

## Success Cases

When a request is successful, the API will return one of the following HTTP status codes:

- 200 OK: The request was successful, and data is returned as a response.
- 201 CREATED: The request was successful, and a new resource was created.

For some endpoints, a JSON object will be returned with a `status` field indicating the result of the operation:

```
{
  "status": "Result message"
}
```

In other cases, the API will return the requested data in JSON format.

## Conclusion

This API provides a simple way to manage signers and their associated keys using a SQLite database. It exposes a set of RESTful endpoints that can be easily integrated into client applications, and can be extended with additional features or adapted to different database systems as needed.