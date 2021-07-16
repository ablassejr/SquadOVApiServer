data "aws_rds_certificate" "rds_cert" {
    latest_valid_till = true
}

resource "aws_db_parameter_group" "primary_db_parameters" {
    name = "primary-squadov-db-parameters"
    family = "postgres12"

    parameter {
        name = "maintenance_work_mem"
        value = "1048576"
    }
}

resource "aws_db_subnet_group" "primary_db_subnet" {
    name = "primary-squadov-db-subnet-group"
    subnet_ids = var.postgres_db_subnets
}

resource "aws_db_instance" "primary_db" {
    allocated_storage = var.postgres_db_size
    apply_immediately = false
    max_allocated_storage = var.postgres_max_db_size
    backup_retention_period = 14
    backup_window = "02:00-02:30"
    ca_cert_identifier = data.aws_rds_certificate.rds_cert.id
    db_subnet_group_name = aws_db_subnet_group.primary_db_subnet.name
    delete_automated_backups = false
    deletion_protection = true
    enabled_cloudwatch_logs_exports = [ "postgresql", "upgrade" ]
    engine = "postgres"
    engine_version = "12.7"
    identifier = var.postgres_instance_name
    instance_class = var.postgres_instance_type
    name = "squadov"
    password = var.postgres_password
    parameter_group_name = aws_db_parameter_group.primary_db_parameters.name
    publicly_accessible = true
    storage_encrypted = true
    storage_type = "gp2"
    username = var.postgres_user
    vpc_security_group_ids = var.postgres_db_security_groups
}