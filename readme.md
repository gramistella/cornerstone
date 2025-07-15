
# Cornerstone: A Production-Ready, Full-Stack Rust Template

![Rust CI](https://github.com/gramistella/cornerstone/actions/workflows/ci.yml/badge.svg)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

Cornerstone is a batteries-included, production-ready template for full-stack Rust applications. It provides a robust, modern, and memory-safe stack, allowing you to skip the boilerplate and focus immediately on writing business logic.

The core philosophy is to provide a solid foundation with sane defaults for a complete application, including a flexible backend, a choice of frontends, database interaction, authentication, and deployment tooling.

---

## ✨ Key Features

*   **Robust Backend**: Built on `axum` for ergonomic and modular web services, with `sqlx` for compile-time checked SQL queries.
*   **Flexible Database**: Out-of-the-box support for **PostgreSQL** and **SQLite**, selectable via feature flags.
*   **Dual Frontend Options**:
    *   **SvelteKit (Web)**: A modern, fast web framework for rich user interfaces, with type-safe API generation from your Rust code.
    *   **Slint (Desktop/WASM)**: A declarative UI toolkit for building native desktop applications in the same Rust ecosystem.
*   **Secure Authentication**: A complete JWT-based authentication system with an access and refresh token rotation strategy.
*   **Developer-First Tooling**:
    *   **`just`**: A command runner for streamlined project tasks (build, test, run).
    *   **Docker**: Multi-stage `Dockerfile` and `docker-compose` for optimized, production-ready containers.
    *   **GitHub Actions**: CI pipeline that tests against both PostgreSQL and SQLite.
    *   **`pre-commit`**: Git hooks for automatic formatting and linting.

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
    *   In `.github/workflows/ci.yml`, delete the entire job for the database you are not using (e.g., `test-sqlite`).

By following these steps, you will have a much cleaner and more focused codebase tailored specifically to your project's needs.

</details>

---

## 🚀 Getting Started

### Prerequisites

*   **Rust Toolchain**: Install via [rustup](https://rustup.rs/).
*   **`just`**: A command runner. Install with `cargo install just`.
*   **`sqlx-cli`**: For database migrations. Install with `cargo install sqlx-cli --no-default-features --features native-tls,rustls,sqlite,postgres`.
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

## 🛠️ Development Workflow

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
*   `just db-reset-sqlite`: Delete and recreate the local SQLite database.

---

## 🐳 Deployment with Docker

A multi-stage `Dockerfile` is provided to build a minimal, optimized production image.

1.  **Build the image:**
    Ensure your `.env` file is configured, as it's used during the build process.
    ```bash
    docker-compose build
    ```

2.  **Run the container:**
    ```bash
    docker-compose up
    ```
The service will be available at `http://localhost:8080`.

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
