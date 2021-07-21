
/*
resource "aws_s3_bucket" "log_storage_bucket" {
    bucket = "squadov-us-log-bucket${var.bucket_suffix}"
    acl = "private"
    tags = {
        "s3" = "logs"
    }
}

resource "aws_cloudfront_origin_access_identity" "private_vod_access_identity" {    
}

resource "aws_cloudfront_origin_access_identity" "public_vod_access_identity" {    
}
*/

resource "aws_s3_bucket" "vod_storage_bucket" {
    bucket = "squadov-us-vod-bucket${var.bucket_suffix}"
    acl = "private"

    lifecycle_rule {
        enabled = true
        abort_incomplete_multipart_upload_days = 1
        transition {
            days = 30
            storage_class = "STANDARD_IA"
        }
    }

    server_side_encryption_configuration {
        rule {
            apply_server_side_encryption_by_default {
                sse_algorithm = "AES256"
            }
        }
    }

    tags = {
        "s3" = "vods"
    }
}

/*
locals {
    private_s3_origin_id = "privateVodS3Origin"
}

resource "aws_cloudfront_distribution" "private_vod_distribution" {
    default_cache_behavior {
        allowed_methods = [ "OPTIONS", "HEAD", "GET" ]
        cached_methods = [ "GET", "HEAD" ]
        target_origin_id = local.private_s3_origin_id

        min_ttl = 600
        default_ttl = 3600
        max_ttl = 86400

        forwarded_values {
            query_string = true
            headers = "*"
            cookies {
                forward = "none"
            }
        }

        viewer_protocol_policy = "redirect-to-https"
    }

    enabled = true
    is_ipv6_enabled = true

    logging_config {
        bucket = aws_s3_bucket.log_storage_bucket.bucket_domain_name
        include_cookies = false
        prefix = "vods_private/"
    }

    origin {
        domain_name = aws_s3_bucket.vod_storage_bucket.bucket_domain_name
        origin_id = local.private_s3_origin_id

        s3_origin_config {
            origin_access_identity = aws_cloudfront_origin_access_identity.private_vod_access_identity.cloudfront_access_identity_path
        }
    }

    price_class = "PriceClass_All"

    tags = {
        "cloudfront" = "privateVod"
    }

    viewer_certificate {
        cloudfront_default_certificate = true
    }
}
*/

resource "aws_s3_bucket" "blob_storage_bucket" {
    bucket = "squadov-us-blob-bucket${var.bucket_suffix}"
    acl = "private"

    lifecycle_rule {
        enabled = true
        abort_incomplete_multipart_upload_days = 1
        transition {
            days = 30
            storage_class = "STANDARD_IA"
        }
    }

    server_side_encryption_configuration {
        rule {
            apply_server_side_encryption_by_default {
                sse_algorithm = "AES256"
            }
        }
    }

    tags = {
        "s3" = "blobs"
    }
}