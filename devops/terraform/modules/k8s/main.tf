resource "google_container_cluster" "primary" {
    name = "squadov-primary-cluster"
    location = "us-central1-c"

    remove_default_node_pool =  true
    initial_node_count = 1
}

resource "google_compute_global_address" "squadov-static-ip" {
    name = "squadov-static-ip"
}

resource "google_container_node_pool" "primary_wow_worker_nodes" {
    name = "squadov-primary-wow-worker-nodes"
    location = "us-central1-c"
    cluster = google_container_cluster.primary.name
    initial_node_count =  1
    lifecycle {
        ignore_changes = [
            initial_node_count
        ]
    }
    
    node_config {
        labels = {
            task = "wowkafka"
        }
        disk_size_gb = 20
        disk_type = "pd-ssd"
        machine_type = "e2-custom-2-3072"
        image_type = "COS"
    }

    autoscaling {
        min_node_count = 1
        max_node_count = 2
    }
}

resource "google_container_node_pool" "vod_worker_nodes" {
    name = "squadov-primary-vod-worker-nodes"
    location = "us-central1-c"
    cluster = google_container_cluster.primary.name
    initial_node_count =  1
    lifecycle {
        ignore_changes = [
            initial_node_count
        ]
    }
    
    node_config {
        labels = {
            task = "vod"
        }
        disk_size_gb = 50
        disk_type = "pd-ssd"
        machine_type = "e2-custom-4-20480"
        image_type = "COS"
    }

    autoscaling {
        min_node_count = 1
        max_node_count = 8
    }
}

resource "google_container_node_pool" "static_nodes" {
    name = "squadov-primary-static-nodes"
    location = "us-central1-c"
    cluster = google_container_cluster.primary.name
    initial_node_count =  1
    lifecycle {
        ignore_changes = [
            initial_node_count
        ]
    }
    
    node_config {
        labels = {
            task = "static"
        }
        disk_size_gb = 20
        disk_type = "pd-ssd"
        machine_type = "n1-custom-1-1024"
        image_type = "COS"
    }

    autoscaling {
        min_node_count = 1
        max_node_count = 2
    }
}

resource "google_container_node_pool" "infra_nodes" {
    name = "squadov-primary-infra-nodes"
    location = "us-central1-c"
    cluster = google_container_cluster.primary.name
    initial_node_count =  1
    lifecycle {
        ignore_changes = [
            initial_node_count
        ]
    }
    
    node_config {
        labels = {
            task = "infra"
        }
        disk_size_gb = 20
        disk_type = "pd-ssd"
        machine_type = "e2-custom-4-4096"
        image_type = "COS"
    }

    autoscaling {
        min_node_count = 1
        max_node_count = 2
    }
}

resource "google_container_node_pool" "demo_worker_nodes" {
    name = "squadov-primary-demo-worker-nodes"
    location = "us-central1-c"
    cluster = google_container_cluster.primary.name
    initial_node_count =  1
    lifecycle {
        ignore_changes = [
            initial_node_count
        ]
    }
    
    node_config {
        labels = {
            task = "demo"
        }
        disk_size_gb = 20
        disk_type = "pd-ssd"
        machine_type = "n1-custom-1-1024"
        image_type = "COS"
    }

    autoscaling {
        min_node_count = 1
        max_node_count = 8
    }
}