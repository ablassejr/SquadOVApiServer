resource "aws_iam_role" "eks_role" {
    name = "eks-cluster-role"
    force_detach_policies = true

    assume_role_policy = <<POLICY
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Effect": "Allow",
            "Principal": {
                "Service": [
                    "eks.amazonaws.com",
                    "eks-fargate-pods.amazonaws.com"
                ]
            },
            "Action": "sts:AssumeRole"
        }
    ]
}
POLICY
}

resource "aws_iam_role_policy_attachment" "eks_role_AmazonEKSClusterPolicy" {
    policy_arn = "arn:aws:iam::aws:policy/AmazonEKSClusterPolicy"
    role       = aws_iam_role.eks_role.name
}

# Optionally, enable Security Groups for Pods
# Reference: https://docs.aws.amazon.com/eks/latest/userguide/security-groups-for-pods.html
resource "aws_iam_role_policy_attachment" "eks_role_AmazonEKSVPCResourceController" {
    policy_arn = "arn:aws:iam::aws:policy/AmazonEKSVPCResourceController"
    role       = aws_iam_role.eks_role.name
}

resource "aws_iam_role_policy_attachment" "eks_role_AmazonEKSFargatePodExecutionRolePolicy" {
    policy_arn = "arn:aws:iam::aws:policy/AmazonEKSFargatePodExecutionRolePolicy"
    role       = aws_iam_role.eks_role.name
}

resource "aws_kms_key" "primary_eks_kms_key" {
    description = "KMS Key: Primary EKS Cluster"
    customer_master_key_spec = "SYMMETRIC_DEFAULT"
}

resource "aws_eks_cluster" "primary_eks_cluster" {
    name = "primary-eks-cluster"
    role_arn = aws_iam_role.eks_role.arn

    vpc_config {
        subnet_ids = var.k8s_subnets
    }

    enabled_cluster_log_types = ["api", "audit", "scheduler"]
    encryption_config {
        provider {
            key_arn = aws_kms_key.primary_eks_kms_key.arn
        }

        resources = ["secrets"]
    }

    version = "1.20"

    depends_on = [
        aws_iam_role_policy_attachment.eks_role_AmazonEKSClusterPolicy,
        aws_iam_role_policy_attachment.eks_role_AmazonEKSVPCResourceController,
    ]
}

data "tls_certificate" "primary_eks_tls_certificate" {
    url = aws_eks_cluster.primary_eks_cluster.identity[0].oidc[0].issuer
}

resource "aws_iam_openid_connect_provider" "primary_eks_iam_provider" {
    url = aws_eks_cluster.primary_eks_cluster.identity[0].oidc[0].issuer

    client_id_list = [
        "sts.amazonaws.com"
    ]

    thumbprint_list = [
        data.tls_certificate.primary_eks_tls_certificate.certificates[0].sha1_fingerprint
    ]
}


resource "aws_eks_fargate_profile" "primary_eks_default_fargate_profile" {
    cluster_name = aws_eks_cluster.primary_eks_cluster.name
    fargate_profile_name = "default-fargate-profile"
    pod_execution_role_arn = aws_iam_role.eks_role.arn
    subnet_ids = var.default_fargate_subnets

    selector {
        namespace = "default"
    }
}

resource "aws_eks_fargate_profile" "primary_eks_coredns_fargate_profile" {
    cluster_name = aws_eks_cluster.primary_eks_cluster.name
    fargate_profile_name = "coredns-fargate-profile"
    pod_execution_role_arn = aws_iam_role.eks_role.arn
    subnet_ids = var.default_fargate_subnets

    selector {
        namespace = "kube-system"
        labels = {
            "k8s-app" = "kube-dns"
        }
    }
}