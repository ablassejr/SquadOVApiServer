resource "google_container_cluster" "primary" {
    name = "squadov-primary-cluster"
    location = "us-central1-c"

    remove_default_node_pool =  true
    initial_node_count = 1
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
        machine_type = "e2-custom-8-16384"
        image_type = "COS"
    }

    autoscaling {
        min_node_count = 1
        max_node_count = 1
    }
}