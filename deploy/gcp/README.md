# Terraform GCP Cloud Function with Docker Container and Volume

This Terraform configuration deploys a Google Cloud Function (2nd generation) with the following features:
- HTTP trigger for function invocation
- Docker container image for the function code
- Dedicated persistent volume for data storage

## Prerequisites

1. Google Cloud Project with billing enabled
2. Terraform installed (v1.0.0+)
3. Docker container image pushed to Google Container Registry or Artifact Registry
4. Google Cloud SDK installed and configured

## Usage

1. Configure your GCP credentials:
```shell
gcloud auth application-default login
```

2. Initialize Terraform:
```shell
terraform init
```

3. Create a `terraform.tfvars` file with your specific values:
```hcl
project_id     = "your-gcp-project-id"
region         = "us-central1"
function_name  = "my-http-function"
container_image = "gcr.io/your-project-id/function-image:latest"
```

4. Deploy the resources:
```shell
terraform plan
terraform apply
```

## Volume Mount

The dedicated volume is mounted at `/mnt/data` in the container by default. You can change this by modifying the `mount_path` variable.

## Important Notes

- The HTTP endpoint is publicly accessible by default. Modify the `google_cloud_run_service_iam_member` resource to restrict access.
- The volume is region-specific and requires the function to run in the same region.
- The function uses Cloud Functions 2nd generation, which is built on Cloud Run.

## Clean Up

To destroy all resources:
```shell
terraform destroy
```
