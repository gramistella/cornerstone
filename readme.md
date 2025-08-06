
# Cornerstone: A Production-Ready, Full-Stack Rust Template

![Rust CI](https://github.com/gramistella/cornerstone/actions/workflows/ci.yml/badge.svg)
[![Rust Version](https://img.shields.io/badge/rust-1.88.0%2B-blue.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Cornerstone is a batteries-included, production-ready template for full-stack Rust applications. It provides a robust, modern, and memory-safe stack, allowing you to skip the boilerplate and focus immediately on writing business logic.

The core philosophy is to provide a solid foundation with sane defaults for a complete application, including a flexible backend, a choice of frontends, database interaction, authentication, and deployment tooling.

---

## 📚 Table of Contents

- [✨ Key Features](#-key-features)
- [🔪 Making It Your Own](#-making-it-your-own)
- [🚀 Getting Started](#-getting-started)
- [⚙️ Configuration](#️-configuration)
- [🛠️ Development Workflow](#️-development-workflow)
- [🐳 Deployment with Docker](#-deployment-with-docker)
- [🔄 Continuous Integration](#-continuous-integration)
- [🏗️ Project Structure](#️-project-structure)
- [⚖️ License](#️-license)

---

## ✨ Key Features

*   **Robust Backend**: Built on `axum` for ergonomic and modular web services, with `sqlx` for compile-time checked SQL queries.
*   **Flexible Database**: Out-of-the-box support for **PostgreSQL** and **SQLite**, selectable via feature flags.
*   **Dual Frontend Options**:
    *   **SvelteKit (Web)**: A modern, fast web framework for rich user interfaces, with type-safe API generation from your Rust code.
    *   **Slint (Desktop/WASM)**: A declarative UI toolkit for building native desktop applications in the same Rust ecosystem.
*   **Secure Authentication**: A complete JWT-based authentication system with an access and refresh token rotation strategy.
* **Automatic API Documentation**: Generated OpenAPI (Swagger) documentation via `utoipa` for easy API testing and exploration.
* **API Rate Limiting**: Protects your application from abuse with configurable, per-IP rate limiting using `tower_governor`.
*   **Developer-First Tooling**:
    *   **`just`**: A command runner for streamlined project tasks (build, test, run).
    *   **Docker**: Multi-stage `Dockerfile` and `docker-compose` for optimized, production-ready containers.
    *   **GitHub Actions**: CI pipeline that tests against both PostgreSQL and SQLite.
    *   **`pre-commit`**: Git hooks for automatic formatting and linting.

---

## 🏛️ Architecture Overview

Here is a high-level overview of the project's components and how they interact:

```mermaid
graph TD
    User[👤 User/Developer]

    subgraph Frontend["🖥️ Frontend Options"]
        Svelte[SvelteKit Web App]
        Slint[Slint Desktop/WASM]
    end

    subgraph Backend["⚙️ Backend (Rust)"]
        Axum[Axum Web Server]
        SQLx[SQLx Query Engine]
    end

    subgraph Database["💾 Database Options"]
        Postgres[PostgreSQL]
        SQLite[SQLite]
    end

    User --> Frontend
    Frontend --> Axum
    Axum --> SQLx
    SQLx --> Database

    classDef frontend fill:#e3f2fd,stroke:#1976d2,stroke-width:2px
    classDef backend fill:#fff3e0,stroke:#f57c00,stroke-width:2px
    classDef database fill:#f3e5f5,stroke:#7b1fa2,stroke-width:2px
    classDef user fill:#e8f5e8,stroke:#388e3c,stroke-width:2px

    class Svelte,Slint frontend
    class Axum,SQLx backend
    class Postgres,SQLite database
    class User user
```

---

## 🔪 Making It Your Own

A template's primary purpose is to be changed. Once you've chosen your stack, it's highly recommended to remove the unused code to simplify your project.

<details>
<summary><strong>Click here for a step-by-step guide to tailoring the template.</strong></summary>

### Part 1: Choosing Your Frontend

Decide whether you will use **SvelteKit** for a web application or **Slint** for a desktop/WASM application, and then follow the steps to remove the other.

#### Option A: I want to use SvelteKit (and remove Slint)

This is the most common path for web applications.

1.  **Delete the Slint Crate:**
    *   Delete the entire `frontend_slint/` directory.

2.  **Update Workspace Configuration:**
    *   In the root `Cargo.toml`, remove `frontend_slint` from the `[workspace].members` array.
    ```diff
    # Cargo.toml
    [workspace]
    resolver = "2"
    members = [
        "backend",
    -   "frontend_slint",
        "common",
    ]
    ```

3.  **Clean Up Backend Features:**
    *   In `backend/Cargo.toml`, you can remove the `slint-ui` feature entirely.
    ```diff
    # backend/Cargo.toml
    [features]
    - default = ["svelte-ui", "db-sqlite"]
    + default = ["svelte-ui", "db-sqlite"] # Ensure this is correct for your DB
    svelte-ui = []
    - slint-ui = []
    # ...
    ```

4.  **Simplify the Backend Web Server:**
    *   In `backend/src/web_server.rs`, the `create_static_router` function has conditional compilation. You can remove the `#[cfg(feature = "slint-ui")]` block and the surrounding logic.

5.  **Clean the `justfile`:**
    *   Remove Slint-specific commands like `build-slint`.
    *   Simplify the `copy-frontend` and `run-web` commands by removing the `slint` conditions.

#### Option B: I want to use Slint (and remove SvelteKit)

This is the path for a desktop-focused application.

1.  **Delete SvelteKit Project:**
    *   Delete the entire `frontend_svelte/` directory.

2.  **Clean Up Backend Features:**
    *   In `backend/Cargo.toml`, remove the `svelte-ui` feature.
    ```diff
    # backend/Cargo.toml
    [features]
    - default = ["svelte-ui", "db-sqlite"]
    + default = ["slint-ui", "db-sqlite"] # Ensure this is correct for your DB
    - svelte-ui = []
    slint-ui = []
    # ...
    ```

3.  **Simplify the Backend Web Server:**
    *   Follow the same logic as in Option A, but keep the `slint-ui` part and remove the `svelte-ui` part in `backend/src/web_server.rs`.

4.  **Remove Type Generation:**
    *   The TypeScript type generation is only for SvelteKit.
    *   Delete `common/src/bin/type_exporter.rs`.
    *   In `common/Cargo.toml`, remove the `type-exporter` binary, the `ts-rs` and `dprint-plugin-typescript` dependencies, and the `ts_export` feature.
    *   In the `justfile`, remove the `gen-types` command.

5.  **Clean the `justfile`:**
    *   Remove Svelte-specific commands: `build-svelte`, `run-web svelte`, `run-web svelte-live`.
    *   Simplify the `copy-frontend` and `run-web` commands.

6.  **Clean the `Dockerfile` and CI:**
    *   Remove all `npm` related steps from the `Dockerfile` and the CI workflow in `.github/workflows/ci.yml`.

---

### Part 2: Choosing Your Database

The process is the same whether you keep PostgreSQL or SQLite. The following example assumes you are **keeping PostgreSQL** and removing SQLite.

1.  **Update Backend Features:**
    *   In `backend/Cargo.toml`, remove the `db-sqlite` feature and update the `default` list.
    ```diff
    # backend/Cargo.toml
    [features]
    - default = ["svelte-ui", "db-sqlite"]
    + default = ["svelte-ui", "db-postgres"]
    # ...
    - db-sqlite = ["sqlx/sqlite", "common/db-sqlite"]
    db-postgres = ["sqlx/postgres", "common/db-postgres"]
    ```

2.  **Update Common Crate Features:**
    *   In `common/Cargo.toml`, remove the `db-sqlite` feature.

3.  **Simplify Database Code:**
    *   The file `backend/src/db.rs` contains conditional logic. You can reduce it to only the `use` statement for your chosen database.
    *   Simplify `backend/build.rs` and `backend/src/main.rs` by removing the `#[cfg]` blocks for the database you are not using.

4.  **Delete Unused Migrations:**
    *   Delete the directory for the database you are not using (e.g., `backend/migrations/sqlite/`).

5.  **Clean the `justfile`:**
    *   Remove all commands related to the unused database (e.g., `db-migrate-sqlite`, `test-backend-sqlite`, `db-reset-sqlite`).

6.  **Clean the CI Workflow:**
    *  The CI pipeline runs tests against both databases. After removing one, you should update the workflow to only test against your chosen database. See Part 3 below for a detailed guide.


-----

### Part 3: Simplifying the CI Workflow

The CI pipeline in the template is configured to test against both PostgreSQL and SQLite using a matrix strategy. Once you've chosen a database, you can simplify the workflow by removing the unused database from the matrix. This is a small change that makes your CI pipeline faster.

#### Option A: I want to use PostgreSQL (and remove SQLite)

1.  **Edit the CI Workflow:**
      * In `.github/workflows/ci.yml`, find the `matrix` within the `test` job.

      * Remove the entire list item for `SQLite`.

        ```diff
        # .github/workflows/ci.yml
        # ...
          test:
            name: Test (${{ matrix.db.name }})
            runs-on: ubuntu-latest
            needs: check-format
            strategy:
              fail-fast: false
              matrix:
                db:
        -         - name: SQLite
        -           type: sqlite
        -           sqlx_features: native-tls,sqlite
        -           url: "sqlite:test_ci.db"
                  - name: PostgreSQL
                    type: postgres
                    sqlx_features: native-tls,postgres
                    url: "postgres://postgres:password@localhost:5432/testdb"
        # ...
        ```
2.  **(Optional) Clean Up Steps:**
      * You can now safely delete the step named `"Create SQLite Database File"`, since it is no longer needed.

        ```diff
        # .github/workflows/ci.yml
        # ...
              - name: Cache Cargo dependencies
                # ...

        -     - name: Create SQLite Database File
        -       if: matrix.db.type == 'sqlite'
        -       run: touch test_ci.db

              - name: Run Database Migrations
                run: just db-migrate-${{ matrix.db.type }}
        # ...
        ```

-----

#### Option B: I want to use SQLite (and remove PostgreSQL)

1.  **Edit the CI Workflow:**
      * In `.github/workflows/ci.yml`, find the `matrix` within the `test` job and remove the list item for `PostgreSQL`.

      * Since PostgreSQL is no longer used, you **must** also remove the entire `services` block that defines the postgres container.

    ```diff
    # .github/workflows/ci.yml
    # ...
        test:
        name: Test (${{ matrix.db.name }})
        runs-on: ubuntu-latest
        needs: check-format
        strategy:
            fail-fast: false
            matrix:
            db:
                - name: SQLite
                type: sqlite
                sqlx_features: native-tls,sqlite
                url: "sqlite:test_ci.db"
    -         - name: PostgreSQL
    -           type: postgres
    -           sqlx_features: native-tls,postgres
    -           url: "postgres://postgres:password@localhost:5432/testdb"

        # Since the PostgreSQL matrix entry is gone, this service is no longer needed.
    -   services:
    -     postgres:
    -       image: postgres:15
    -       env:
    -         POSTGRES_USER: postgres
    -         POSTGRES_PASSWORD: password
    -         POSTGRES_DB: testdb
    -       ports:
    -         - 5432:5432
    -       options: >-
    -         --health-cmd pg_isready
    -         --health-interval 10s
    -         --health-timeout 5s
    -         --health-retries 5

        env:
            # Set the DATABASE_URL for sqlx-cli and the application tests
    # ...
    ```

</details>

---

## 🚀 Getting Started

### Prerequisites

*   **Rust Toolchain**: Install via [rustup](https://rustup.rs/).
*   **`just`**: A command runner. Install with `cargo install just`.
*   **`sqlx-cli`**: For database migrations. Install with `cargo install sqlx-cli --no-default-features --features native-tls,sqlite,postgres`.
     > **Note**: This command installs `sqlx-cli` with support for both database types, which is useful when you first start. After you've chosen a database and removed the other, you can use a simpler command (e.g., `cargo install sqlx-cli --no-default-features --features native-tls,postgres`).
*   **Node.js & npm**: Required for the SvelteKit frontend.
*   **Docker & Docker Compose**: (Optional) For running the application in a container.
*   **`pre-commit`**: (Optional) For automatic git hooks. Install from [pre-commit.com](https://pre-commit.com/).

### Installation & Setup

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/YOUR_USERNAME/cornerstone.git
    cd cornerstone
    ```

2.  **Configure Environment:**
    Copy the example `.env` file. This file is ignored by git and is used for local secrets.
    ```bash
    cp .env.example .env
    ```
    Open `.env` and set a strong `APP_JWT__SECRET`. Ensure the `DATABASE_URL` points to your chosen database.

3.  **Install Frontend Dependencies (for SvelteKit):**
    ```bash
    cd frontend_svelte
    npm install
    cd ..
    ```

4.  **Setup the Database:**
    The project defaults to SQLite. To run the initial migrations for it:
    ```bash
    just db-migrate-sqlite
    ```
    *(Use `just db-migrate-postgres` if you've switched to PostgreSQL).*

5.  **(Optional) Install Git Hooks:**
    This will run formatting and linting checks before each commit.
    ```bash
    pre-commit install
    ```

---

## ⚙️ Configuration

Application configuration is handled through a combination of a configuration file and environment variables, powered by the `figment` crate.

* **`Config.toml`**: This file contains non-sensitive, default configuration values. You can commit this file to version control.
* **`.env`**: This file is for local development secrets and environment-specific overrides. **It is ignored by git and should never be committed.**
* **Environment Variables**: Any environment variable prefixed with `APP_` will override values from both `Config.toml` and `.env`. The double underscore `__` is used to denote nesting (e.g., `APP_WEB__PORT=8888` overrides the `port` key within the `[web]` table).

The hierarchy of overrides is: **Environment Variables > `.env` file > `Config.toml`**.

### Key Configuration Variables

> The application **will not start** without a valid `APP_JWT__SECRET` set in your environment. This is a critical security measure. Ensure you set it in your `.env` file after copying it from `.env.example`.

* `APP_JWT__SECRET`: **(Required)** A long, random string used to sign JWTs. This **must** be set in your `.env` file or as an environment variable for production.
* `DATABASE_URL`: The connection string for your primary database. This is used by `sqlx-cli` for migrations and by the application at runtime. For Docker builds, this value is passed in during the build process (see `docker-compose.yml`).
* `DATABASE_URL_SQLITE`: A separate variable for the SQLite connection string, used by `just` commands.

---

## 🛠️ Development Workflow

### API Documentation

This template uses `utoipa` to automatically generate an OpenAPI (Swagger) specification for the backend API. When running in debug mode, you can access the interactive Swagger UI at:

* **[http://localhost:8080/docs](http://localhost:8080/docs)**

The documentation is generated directly from the Rust code via the `#[utoipa::path]` macros on your API handlers (e.g., in `backend/src/web_server.rs` and `backend/src/auth.rs`). When you change an endpoint, remember to update its corresponding macro to keep the documentation in sync.

This project uses `just` as a command runner for common tasks.

### Web Development (SvelteKit with Hot-Reloading)

This is the recommended way to develop the web application. It runs the backend server and the SvelteKit dev server concurrently.

```bash
# Frontend (with HMR): http://localhost:5173
# Backend API: http://localhost:8080
just run-web svelte-live
```

### Web Development (Production Simulation)

To build the static SvelteKit app and have the Rust server serve it, simulating a production environment:

```bash
# Access the full app at http://localhost:8080
just run-web svelte
```

### Desktop Development (Slint)

To build and run the native Slint desktop application:

```bash
just run-web slint
```

### Other Useful Commands

*   `just test`: Run the entire Rust test suite.
*   `just lint`: Check the workspace for warnings and errors with Clippy.
*   `just gen-types`: **Important!** Regenerate TypeScript types in `frontend_svelte` after changing shared Rust structs in the `common` crate.
+ * `just db-prepare`: **Highly Recommended!** Checks all SQL queries in the `backend` against a running database to ensure they are valid at compile time.
*   `just db-reset-sqlite`: Delete and recreate the local SQLite database.

---

## 🐳 Deployment with Docker

A multi-stage `Dockerfile` is provided to build a minimal, optimized production image. It's designed to be flexible and efficient.

### How it Works

1.  **Builder Stage**: This stage sets up the complete build environment, installing Rust, Node.js, `sqlx-cli`, and other dependencies.
2.  **Optimized Caching**: It caches `npm` and `cargo` dependencies in separate layers to dramatically speed up subsequent builds.
3.  **Dynamic Build**: The Dockerfile dynamically determines which frontend and database to use by inspecting the `default` features in `backend/Cargo.toml`. This means you don't have to edit the `Dockerfile` after removing an unused frontend or database!
4.  **Runtime Stage**: This final stage creates a tiny production image by copying *only* the compiled server binary and necessary static assets from the builder stage.

### Building and Running

1.  **Build the image:**
    Ensure your `.env` file is configured, as its variables are used during the build.
    ```bash
    docker-compose build
    ```

2.  **Run the container:**
    ```bash
    docker-compose up
    ```
The service will be available at `http://localhost:8080`.

---

## 🔄 Continuous Integration

The project includes a comprehensive CI pipeline using **GitHub Actions**, defined in `.github/workflows/ci.yml`.

On every push or pull request to the `main` branch, the CI pipeline automatically performs the following checks:

1.  **Formatting**: Ensures all Rust code adheres to the standard `rustfmt` style (`cargo fmt --check`).
2.  **Matrix Testing**: It runs two parallel test jobs to validate the application against both supported databases:
    * **PostgreSQL**: A `postgres:15` service is spun up within the job to run tests against a real database instance.
    * **SQLite**: Tests are run against a file-based SQLite database.
3.  **Linting**: Runs `cargo clippy` to catch common mistakes and improve code quality for each database configuration.
4.  **Backend Tests**: Executes the full integration test suite (`cargo test -p backend`).
5.  **Release Build**: Compiles the entire workspace in release mode to ensure it builds successfully for production.

This setup guarantees that your application remains robust and compatible with its supported database configurations. Once you have chosen a specific database for your project, you can simplify this workflow. See the ["Making It Your Own"](#-making-it-your-own) section for a guide.

---

## 🏗️ Project Structure

The project is a Cargo workspace with a clean separation of concerns.

```
cornerstone/
├── .github/             # GitHub Actions CI workflows
├── backend/             # The Rust Axum web server
│   ├── migrations/      # SQLx database migrations
│   ├── src/             # Backend source code
│   └── static/          # Where the built frontend is served from
├── common/              # Shared Rust code (DTOs, utils)
├── frontend_slint/      # The Slint desktop frontend crate
├── frontend_svelte/     # The SvelteKit web frontend project
│   └── src/lib/types.ts # Auto-generated types from Rust!
├── .env                 # Local environment variables (ignored by git)
├── Config.toml          # Default application configuration
├── justfile             # Command runner recipes
└── Dockerfile           # For building a production container image
```

---

## ⚖️ License

This project is licensed under the **MIT License**. See the `LICENSE` file for details.
