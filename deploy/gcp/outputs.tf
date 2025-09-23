output "function_uri" {
  description = "The URI of the deployed function"
  value       = google_cloudfunctions2_function.function.service_config[0].uri
}

output "function_name" {
  description = "Name of the deployed function"
  value       = google_cloudfunctions2_function.function.name
}

output "function_status" {
  description = "Status of the deployed function"
  value       = google_cloudfunctions2_function.function.state
}

output "volume_id" {
  description = "ID of the created disk volume"
  value       = google_compute_disk.function_disk.id
}

output "service_account" {
  description = "Service account used by the function"
  value       = local.service_account_email
}
