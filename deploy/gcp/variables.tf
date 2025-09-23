variable "project_id" {
  description = "The Google Cloud project ID"
  type        = string
}

variable "region" {
  description = "The region where resources will be deployed"
  type        = string
  default     = "us-central1"
}

variable "function_name" {
  description = "Name of the Cloud Function"
  type        = string
  default     = "http-container-function"
}

variable "description" {
  description = "Description of the Cloud Function"
  type        = string
  default     = "HTTP triggered Cloud Function using Docker container"
}

variable "container_image" {
  description = "Docker container image URL (e.g., 'gcr.io/project-id/function-image:latest')"
  type        = string
}

variable "service_account_email" {
  description = "Service account email for the Cloud Function"
  type        = string
  default     = null
}

variable "available_memory_mb" {
  description = "Memory available to the function (in MB)"
  type        = number
  default     = 256
}

variable "timeout_seconds" {
  description = "Timeout for the function in seconds"
  type        = number
  default     = 60
}

variable "volume_name" {
  description = "Name of the dedicated volume"
  type        = string
  default     = "function-volume"
}

variable "volume_size_gb" {
  description = "Size of the dedicated volume in GB"
  type        = number
  default     = 10
}

variable "mount_path" {
  description = "Path where the volume will be mounted inside the container"
  type        = string
  default     = "/mnt/data"
}

variable "max_instance_count" {
  description = "Maximum number of function instances"
  type        = number
  default     = 10
}

variable "min_instance_count" {
  description = "Minimum number of function instances (for better cold start)"
  type        = number
  default     = 0
}

variable "ingress_settings" {
  description = "Ingress settings for the function"
  type        = string
  default     = "ALLOW_ALL"
}

variable "vpc_connector" {
  description = "VPC connector for the function (optional)"
  type        = string
  default     = null
}

variable "vpc_connector_egress_settings" {
  description = "VPC connector egress settings"
  type        = string
  default     = "PRIVATE_RANGES_ONLY"
}

variable "environment_variables" {
  description = "Environment variables to pass to the function"
  type        = map(string)
  default     = {}
}

variable "labels" {
  description = "Labels to apply to the function"
  type        = map(string)
  default     = {}
}
