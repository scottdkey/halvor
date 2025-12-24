{{/*
Expand the name of the chart.
*/}}
{{- define "portainer.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "portainer.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "portainer.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "portainer.labels" -}}
helm.sh/chart: {{ include "portainer.chart" . }}
{{ include "portainer.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "portainer.selectorLabels" -}}
app.kubernetes.io/name: {{ include "portainer.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Get the image repository based on deployment type
*/}}
{{- define "portainer.imageRepository" -}}
{{- if .Values.image.repository }}
{{- .Values.image.repository }}
{{- else if eq .Values.deploymentType "agent" }}
{{- "portainer/agent" }}
{{- else if eq .Values.deploymentType "be" }}
{{- "portainer/portainer-ee" }}
{{- else }}
{{- "portainer/portainer-ce" }}
{{- end }}
{{- end }}

{{/*
Get container name based on deployment type
*/}}
{{- define "portainer.containerName" -}}
{{- if eq .Values.deploymentType "agent" }}
{{- "portainer-agent" }}
{{- else }}
{{- "portainer" }}
{{- end }}
{{- end }}

