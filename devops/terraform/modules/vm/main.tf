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