# Under

[![Version](https://img.shields.io/crates/v/under)](https://crates.io/crates/under)
[![docs.rs](https://img.shields.io/docsrs/under)](https://docs.rs/under)
[![License](https://img.shields.io/crates/l/under)](./LICENSE)
[![CI](https://github.com/medcat/under/actions/workflows/main.yml/badge.svg?branch=main)](https://github.com/medcat/under/actions/workflows/main.yml)

A very simple HTTP server framework.  This serves as a small layer
between your application code and Hyper.

Right now, this layer is very bare-bones - however, the intent is to
add on to this whenever patterns with the code become obvious.  If
you encounter any, feel free to create an issue.

## Setting Up

The simplest way to set up a server is by using `under::Router`:

```rust
let mut http = under::http();
http.at("/").get(under::endpoints::simple(under::Response::empty_204));
http.listen("localhost:8080").await.unwrap();
```

This will cause the application to run an HTTP server on port 8080
locally; running `GET /` would return `204 No Content`.

The router accepts all kinds of verbs:

```rust
http.at("/users")
    .get(users::index)
    .post(users::create)
    .at("/{id}")
        .get(users::show)
        .put(users::update)
        .delete(users::delete);
```

Note the hirearchal structure of the `at` calls - if the latter `at`
is called underneath an already existing `at`, then the paths are
joined.  The above example is equiavlent to:

```rust
http.at("/users")
    .get(users::index)
    .post(users::create);
http.at("/users/{id}")
    .get(users::show)
    .put(users::update)
    .delete(users::delete);
```

For verbs that are not included by default, you may use the `method`
function to declare one:

```rust
http.at("/users/{id}")
    .method(hyper::Method::from_bytes(b"SOMETHING").unwrap(), users::something);
```

Or, if you want to capture all methods, you can use the `all` function
todo so:

```rust
http.at("/users/{id}").all(users::all);
```

## Endpoints

When declaring a path, it must route to an endpoint - the argument
passed into the verb function, e.g. `get(users::show)`.  The value
passed _must_ implement `under::Endpoint`.  `under::Endpoint` is implemented for
`fn(under::Request) -> impl Future<Output = Result<under::Response, anyhow::Error> + Send + Sync + 'static`;
so, declaring an endpoint can be as simple as:

```rust
async fn index(request: under::Request) -> Result<under::Response, anyhow::Error> {
    todo!()
}
```

and used as above.  This library also comes with a few built-in
endpoints in `under::endpoints`.
