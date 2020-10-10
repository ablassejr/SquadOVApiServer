# Deploy to GCP

This guide will take you through how to deploy SquadOV to a new environment on GCP on a fresh dev machine.
We will assume the variable `$SRC` points to the git project folder.

## Prerequisites

The instructions in this guide will assume you're running WSL2 (Debian).
See [here](https://docs.microsoft.com/en-us/windows/wsl/install-win10) for instructions on getting that setup.
We'll be using the following tools to deploy:

* [SOPS](https://github.com/mozilla/sops)
* [Ansible](https://www.ansible.com/)
* [Terraform](https://www.terraform.io/)
* [Flyway](https://flywaydb.org/)
* [Google Cloud SDK](https://cloud.google.com/sdk)

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
   6. `sops --gcp-kms projects/$ENV/locations/global/keyRings/sops/cryptoKeys/sops-key $ENV_vars.json`

   Replace `$ENV` with the name of your GCP project.
   This should open up a Vim (or whatever text editor) prompt.
   Remove the default contents and then copy and paste the contents of `$SRC/devops/env/dev_vars.json` into it.
   Set `POSTGRES_USER` and `POSTGRES_PASSWORD` to new (and secure) values.
   Set `FUSIONAUTH_DB_USER` and ``FUSIONAUTH_DB_PASSWORD` to new (and secure) values as well.
   Set `GCP_PROJECT` to `$ENV`.
   We'll setup FusionAuth later.
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
   10. `sops exec-env ../env/$ENV_vars.json './run_terraform.sh $ENV'`
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
    4.  Modify `/etc/ansible/hosts` and add the external IP of the newly created VM instance to a group called `vm`. Make it look like the following.

    ```
    [vm]
    IP_ADDRESS_HERE
    ```
6.  You will now need to enable your machine to SSH into your VM instance in an Ansible friendly way.
    1. `ssh-keygen -t rsa -b 4096 -C $(whoami)`
    2. `cp PUB_FILE GCLOUD_PUB_FILE`. Replace `PUB_FILE` with the public key you just generated.
    3. Open `GCLOUD_PUB_FILE` in an editor and append the username part of your SSH public key to the front.
    e.g. If your key looks like `ssh-rsa KEY_VALUE USERNAME` change it to `USERNAME:sh-rsa KEY_VALUE USERNAME`
    4. `gcloud compute project-info add-metadata --metadata-from-file ssh-keys=GCLOUD_PUB_FILE`
    5. Running `ansible all -m ping` should now succeed.
7.  Go heere in your browser and enable the SQL Admin API: `https://console.developers.google.com/apis/api/sqladmin.googleapis.com/overview?project=$ENV`
8. We are now going to deploy our supporting infrastructure (FusionAuth, connection to the database, etc.) using Ansible.
   1. `cd $SRC/devops/ansible`
   2. `ansible-playbook -v prep_env.yml`
   3. `sops exec-env ../env/$ENV_vars.json 'ansible-playbook -v deploy_supporting_infra.yml'`