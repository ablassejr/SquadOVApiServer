# Getting Started

## Prerequisites

From here on out, the `SquadOVApiServer` folder will be referred to as `$SRC`.

* Install [Docker](https://docs.docker.com/docker-for-windows/wsl/).
* Install [Docker Compose](https://docs.docker.com/compose/install/) (follow the instructions for Linux).

Other, you will need to install additional dependencies in the `deps` folder.

1. `cd $SRC/deps`
2. `.\pull_deps.ps1`
3. Add the following paths to your PATH environment variable:
    * `$SRC/deps/flyway`

## Setting up Infrastructure Services

1. `cd $SRC/config`
2. `cp config.toml.tmpl config.toml`
3. Set the following variables:
   1. `fusionauth.host` to `http://127.0.0.1`
   2. `database.url` to `postgresql://postgres:password@127.0.0.1/squadov`
   3. `cors.domain` to `http://localhost:3000`

### Set up FusionAuth

1. `cd $SRC/devops/docker`
2. `..\env\dev_env.ps1`
3. `docker-compose -f local-dev-compose.yml up`
4. Open up `127.0.0.1:9011` in a browser and setup FusionAuth using the Setup Wizard.
5. Login and create a new application.

    Give the application a reasonable name. Under the Security tab set

    * `Require an API key` to true.
    * `Generate Refresh Tokens` to true.
    * `Enable JWT refresh` to true.

    Hit save.
6. Set `FUSIONAUTH_CLIENT_ID` and `FUSIONAUTH_CLIENT_SECRET` in `$SRC/devops/env/dev_vars.json` to your application's client ID and client secret respectively.
7. Setup a FusionAuth API key.

    Copy the key and modify `FUSIONAUTH_API_KEY` in `$SRC/devops/docker/dev_env.sh` to the key value.
    TODO: Determine minimal set of endpoint permissions.
8. Setup an SMTP server.

    Under General, set:
    * Issuer: squadov.gg

    Under Email, set:

    * Host: smtp.sendgrid.net
    * Port: 587
    * Security: TLS
    * Username: apikey
    * Password: Sendgrid API key
    * Verify Email: TRUE
    * Verify email when changed: TRUE
    * Verification template: Email Verification
    * Delete unverified users: TRUE
9. Set FusionAuth's `api_key`, `tenant_id`, and `application_id` in `$SRC/config/config.toml` to the appropriate value.

### Setup PostgreSQL

1. `cd $SRC/devops/database`
2. `.\migrate.ps1`

### Build and Run

1. `cd $SRC`
2. `cargo install sqlx-cli --no-default-features --features postgres --version 0.1.0-beta.1`
3. `cargo sqlx prepare`
4. `cargo build`
5. `cargo run -- --config .\config\config.toml`