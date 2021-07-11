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
}

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
}