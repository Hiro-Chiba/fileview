terraform {
  required_providers {
    aws = { source = "hashicorp/aws", version = "~> 5.0" }
  }
}

provider "aws" {
  region = var.region
}

resource "aws_s3_bucket" "artifacts" {
  bucket = "nebula-artifacts-${var.env}"
}

resource "aws_ecs_cluster" "workers" {
  name = "nebula-${var.env}"
}
