# Deploy to GCP

This guide will take you through how to deploy SquadOV to a new environment on GCP on a fresh dev machine.
We will assume that you have all the projects checked out:
* `SquadOVApiServer` (referred to as `$SRC`)
* `SquadOVWebApp` (referred to as `$APP`)
* `SquadOVClient` (referred to as `$CLIENT`)

## Prerequisites

The instructions in this guide will assume you're running WSL2 (Debian).
See [here](https://docs.microsoft.com/en-us/windows/wsl/install-win10) for instructions on getting that setup.
We'll be using the following tools to deploy:

* [SOPS](https://github.com/mozilla/sops)
* [Ansible](https://www.ansible.com/)
* [Terraform](https://www.terraform.io/)
* [Flyway](https://flywaydb.org/)
* [Google Cloud SDK](https://cloud.google.com/sdk)

You will need to ensure that you have a few Debian packages installed to complete this guide (list may not be comprehensive)
* curl
* unzip

## New Deployment Guide

1. Setup the Google Cloud SDK command line tools.
   1. `sudo apt-get install apt-transport-https ca-certificates gnupg`
   2. `echo "deb [signed-by=/usr/share/keyrings/cloud.google.gpg] https://packages.cloud.google.com/apt cloud-sdk main" | sudo tee -a /etc/apt/sources.list.d/google-cloud-sdk.list`
   3. `curl https://packages.cloud.google.com/apt/doc/apt-key.gpg | sudo apt-key --keyring /usr/share/keyrings/cloud.google.gpg add -`
   4. `sudo apt-get update && sudo apt-get install google-cloud-sdk`
   5. `gcloud init`

   If you've already performed the `init`, you'll want to use `gcloud auth application-default login` to login for future deployment runs.

   Create a new project that you wish to deploy to.
2. Setup SOPS.
   1. `curl -LO https://github.com/mozilla/sops/releases/download/v3.6.1/sops_3.6.1_amd64.deb`
   2. `sudo dpkg -i sops_3.6.1_amd64.deb`
   3. `cd $SRC/devops/env`
   4. `gcloud kms keyrings create sops --location global`
   5. `gcloud kms keys create sops-key --location global --keyring sops --purpose encryption`
   6. Add this key to `$SRC/.sops.yaml` assuming the vars file is `$ENV_vars.json`
   7. `sops --gcp-kms projects/$ENV/locations/global/keyRings/sops/cryptoKeys/sops-key $ENV_vars.json`

   Replace `$ENV` with the name of your GCP project.
   This should open up a Vim (or whatever text editor) prompt.
   Remove the default contents and then copy and paste the contents of `$SRC/devops/env/dev_vars.json` into it.
   Set the following variables:
    * `POSTGRES_USER` and `POSTGRES_PASSWORD` to new (and secure) values. The Postgres password should avoid special characters (I think the `@` character makes things wonky with the Sqlx Crate for Rust).
    * `POSTGRES_HOST` to `172.22.0.2`
    * `FUSIONAUTH_HOST` to `172.22.0.3`
    * `FUSIONAUTH_DB_USER` and ``FUSIONAUTH_DB_PASSWORD` to new (and secure) values as well.
    * `GCP_PROJECT` to `$ENV`.
    * `GITLAB_USERNAME` to your Gitlab username that is associated with the following personal access token.
    * `GITLAB_REGISTRY_TOKEN` to a personal access token that has the `read_registry` and `write_registry` permissions.
    * `DEPLOYMENT_DOMAIN` to the domain you wish to deploy to (e.g. `staging.squadov.gg`).
    * `DEPLOYMENT_DOMAIN_EMAIL` to the email address that you wish to register the Let's Encrypt certificate with.

   We'll setup the other environment variables later.
   Save the file and verify that the contents of the `$ENV_vars.json` file is encrypted (i.e. `cat $ENV_vars.json`).
3. Setup Terraform.
   1. `curl -O https://releases.hashicorp.com/terraform/0.13.4/terraform_0.13.4_linux_amd64.zip`
   2. `unzip terraform_0.13.4_linux_amd64.zip`
   3. Add the location of the `terraform` binary to your `$PATH` (you should move the terraform binary to a place that makes sense first).
   4. `cd $SRC/devops/terraform`
   5. `mkdir $ENV`
   6. `cp TEMPLATE/*.tf $ENV/`
   7. Replace `GCP_PROJECT` and `GCP_BUCKET` with the Google Cloud project and the Google Cloud bucket where you want to store your terraform state respectively.
   8. If you haven't done so already, create the `GCP_BUCKET` using the Google Cloud console now.
   9. Enable versioning: `gsutil versioning set on gs://GCP_BUCKET`
   10. Enable the Google Compute Engine API by going here: `https://console.developers.google.com/apis/api/compute.googleapis.com/overview?project=${ENV}`
   11. `sops exec-env ../env/$ENV_vars.json './run_terraform.sh $ENV'`
4.  Setup Flyway.
    1.  `curl -O https://repo1.maven.org/maven2/org/flywaydb/flyway-commandline/7.0.2/flyway-commandline-7.0.2-linux-x64.tar.gz`
    2.  `tar xvf flyway-commandline-7.0.2-linux-x64.tar.gz`
    3.  Add the `flyway-7.0.2` folder to your `$PATH`.
    4.  `curl  https://dl.google.com/cloudsql/cloud_sql_proxy.linux.amd64 > cloud_sql_proxy`
    5.  `chmod +x cloud_sql_proxy`
    6.  Add the location of the `cloud_sql_proxy` executable to your `$PATH`.
    7.  `cd $SRC/devops/database`
    8.  `sops exec-env ../env/$ENV_vars.json './migrate.sh'`
5.  Setup Ansible.
    1.  `echo "deb http://ppa.launchpad.net/ansible/ansible/ubuntu trusty main" | sudo tee -a /etc/apt/sources.list`
    2.  `sudo apt-key adv --keyserver keyserver.ubuntu.com --recv-keys 93C4A3FD7BB9C367`
    3.  `sudo apt update && sudo apt install ansible`
    4.  Modify `/etc/ansible/hosts` and add the external IP of the newly created VM instance to a group called `$ENV`. Make it look like the following.

    ```
    [$ENV]
    IP_ADDRESS_HERE
    ```

    Note that you can't have special characters in the group name so change `$ENV` only when dealing with Ansible.
6.  You will now need to enable your machine to SSH into your VM instance in an Ansible friendly way.
    1. `ssh-keygen -t rsa -b 4096 -C $(whoami)`
    2. `cp PUB_FILE GCLOUD_PUB_FILE`. Replace `PUB_FILE` with the public key you just generated.
    3. Open `GCLOUD_PUB_FILE` in an editor and append the username part of your SSH public key to the front.
    e.g. If your key looks like `ssh-rsa KEY_VALUE USERNAME` change it to `USERNAME:sh-rsa KEY_VALUE USERNAME`
    1. `gcloud compute project-info add-metadata --metadata-from-file ssh-keys=GCLOUD_PUB_FILE`
    2. Running `ansible $ENV -m ping` should now succeed.
7. Go here in your browser and enable the SQL Admin API: `https://console.developers.google.com/apis/api/sqladmin.googleapis.com/overview?project=$ENV`
8. Go to Cloudflare.com and add DNS entries for the root, `app`, `auth`, and `api` subdomains to `${DEPLOYMENT_DOMAIN}`. Set the IP address to be the external static IP of the Google Cloud VM you created.
**NOTE**: Unless you are deploying for the root subdomain `squadov.gg`, do not allow Cloudflare to proxy these domains.
9. Before deploying the support infrastructure, we will need to create the NGINX container.
   1. `cd $SRC/devops/docker/nginx`
   2. `sudo  ln -s /mnt/c/Program\ Files/Docker/Docker/resources/bin/docker-credential-desktop.exe /usr/bin/docker-credential-desktop.exe`
   3. `sops exec-env ../../env/$ENV_vars.json './build.sh'`
10. We are now going to deploy our supporting infrastructure (FusionAuth, connection to the database, etc.) using Ansible.
   1. `cd $SRC/devops/ansible`
   2. `ansible-playbook -e "shosts=$ENV" -v prep_env.yml`
   3. You may have to wait a few minutes for Ansible to pick up a new SSH session so that you have access to Docker. Or you can manually find and kill Ansible's SSH process.
   4. `sops exec-env ../env/$ENV_vars.json 'ansible-playbook -e "shosts=$ENV" -v deploy_supporting_infra.yml'`
11. At this point you should be able to go to `https://auth.${DEPLOYMENT_DOMAIN}` and be greeted with the FusionAuth setup screen. Following the instructions from the GETTING_STARTED.md document but set the corresponding variables in your `${ENV}_vars.json` file. Additionally, set the `FUSIONAUTH_TENANT_ID` variable.
12. After you finished doing the standard FusionAuth setup, you will also need to change the `Email Verification` and `Forgot Password` email templates.
    1.  Modify the `Email Verification` email template to direct users to the URL: `https://app.${DEPLOYMENT_DOMAIN}/verify/${verificationId}` in both the HTML Template and the Text Template.
        1.  Change the `Default Subject` to `Verify your SquadOV email address`
    2.  Do the same set of changes for the `Forgot Password` email template but direct users to the URL `https://app.${DEPLOYMENT_DOMAIN}/forgotpw/${changePasswordId}`.

At this point you should have functioning infrastructure for the SquadOV backend to work with so we can deploy the `SquadOVApiServer` and the `SquadOVWebApp`.
First we'll build the `SquadOVApiServer` Docker container.

1. `cd $SRC/devops/build`
2. `sops exec-env ../env/$ENV_vars.json './build.sh'`.

Next we'll build the `SquadOVWebApp` Docker container.

1. `cd $APP`
2. Create a `webpack/${ENV}.config.js` webpack configuration. Copy from another deployment's config and set the `API_URL` to `https://api.${DEPLOYMENT_DOMAIN}`.
3. `./devops/build.sh $ENV`

Finally, we can deploy the API server and the web app.
1. `cd $SRC/devops/ansible`
2. `sops exec-env ../env/$ENV_vars.json 'ansible-playbook -e "shosts=$ENV" -v deploy_web_api_app.yml'`

Now we need to build the desktop client such that it can connect to the services you just deployed.
Go back into a Powershell terminal.

1. `cd $CLIENT`
2. `cp client_ui\webpack\prod.config.js client_ui\webpack\${ENV}.config.js`
3. Set `API_URL` to `https://api.${DEPLOYMENT_DOMAIN}`.
4. `cd .\scripts\windows`
5. `.\package.ps1 $ENV never`. If this is an offical release, change `never` to `always`. If set to `always` you will need to get the `GH_TOKEN` environment variable (i.e. `$env:GH_TOKEN="TOKEN_HERE"; ...`).

You should now see the `SquadOV.exe` executable in `$CLIENT\client_ui\package\win\x64\$VERSION\win-unpacked` where `$VERSION` is whatever the version is in the `package.json`.