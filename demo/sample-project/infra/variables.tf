variable "region" {
  type    = string
  default = "ap-northeast-1"
}

variable "env" {
  type        = string
  description = "Deployment environment (staging or production)"
}
