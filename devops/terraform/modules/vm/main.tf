data "google_compute_image" "vm-image" {
    name = "debian-10-buster-v20200910"
    project = "debian-cloud"
}

resource "google_compute_network" "vm-network" {
    name                    = "vm-network"
    auto_create_subnetworks = false
    routing_mode            = "REGIONAL"
}

resource "google_compute_subnetwork" "vm-network-us-central1" {
    name                    = "vm-network-us-central1"
    region                  = "us-central1"
    network                 = google_compute_network.vm-network.self_link
    ip_cidr_range           = "192.168.1.0/24"
}

resource "google_compute_firewall" "vm-network-ssh-ingress" {
    name                    = "vm-network-ssh-ingress"
    network                 = google_compute_network.vm-network.name
    direction               = "INGRESS"

    allow {
        protocol = "tcp"
        ports = ["22"]
    }
}

resource "google_compute_firewall" "vm-network-http-ingress" {
    name                    = "vm-network-http-ingress"
    network                 = google_compute_network.vm-network.name
    direction               = "INGRESS"

    allow {
        protocol = "tcp"
        ports = ["80", "443"]
    }
}

resource "google_compute_address" "vm-static-ip" {
    name    = "vm-static-ip"
    region  = google_compute_subnetwork.vm-network-us-central1.region
}

resource "google_compute_disk" "vm-boot-disk" {
    name    = "vm-boot-disk"
    image   = data.google_compute_image.vm-image.self_link
    size    = 20
    type    = "pd-standard"
    zone    = "us-central1-c"
}

resource "google_compute_instance" "vm" {
    name                        = "squadov-vm-central1-c"
    machine_type                = "n1-standard-1"
    zone                        = "us-central1-c"
    allow_stopping_for_update   = true

    boot_disk {
        auto_delete = false
        source      = google_compute_disk.vm-boot-disk.self_link
    }

    network_interface {
        network    = google_compute_network.vm-network.self_link
        subnetwork = google_compute_subnetwork.vm-network-us-central1.self_link

        access_config {
            nat_ip = google_compute_address.vm-static-ip.address
        }
    }

    service_account {
        scopes = ["sql-admin"]
    }
}

resource "google_storage_bucket" "vod-storage-bucket" {
    name            = var.vod_storage_bucket
    location        = "US-CENTRAL1"
    storage_class   = "STANDARD"
}

resource "google_storage_bucket" "blob-storage-bucket" {
    name            = var.blob_storage_bucket
    location        = "US-CENTRAL1"
    storage_class   = "STANDARD"
}

resource "google_project_iam_custom_role" "api-service-custom-role" {
    role_id     = "apiServiceAccountRole"
    title       = "API Service Account Role"
    permissions = ["storage.objects.get", "storage.objects.delete", "storage.objects.list", "storage.objects.create", "storage.buckets.get"]
}

resource "google_service_account" "api-service-account" {
    account_id      = "api-service-account"
    display_name    = "API Service Account"
}

resource "google_service_account_key" "api-service-account-key" {
    service_account_id = google_service_account.api-service-account.name
}

resource "google_project_iam_member" "service-account-role" {
    role = google_project_iam_custom_role.api-service-custom-role.name
    member = "serviceAccount:${google_service_account.api-service-account.email}"
}

resource "local_file" "api-service-account-key-file" {
    filename = var.service_account_key_filename
    content = base64decode(google_service_account_key.api-service-account-key.private_key)
}