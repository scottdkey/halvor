{{/*
Expand the name of the chart.
Usage: {{- include "common.name" . }}
Or: {{- include "common.name" (dict "Chart" .Chart "Values" .Values) }}
*/}}
{{- define "common.name" -}}
{{- if .Chart }}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- default .name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}

{{/*
Create a default fully qualified app name.
Usage: {{- include "common.fullname" . }}
Or: {{- include "common.fullname" (dict "Release" .Release "Chart" .Chart "Values" .Values) }}
*/}}
{{- define "common.fullname" -}}
{{- if .Chart }}
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
{{- else }}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
Usage: {{- include "common.chart" . }}
Or: {{- include "common.chart" (dict "Chart" .Chart) }}
*/}}
{{- define "common.chart" -}}
{{- if .Chart }}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .name .version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}

{{/*
Common labels
Usage: {{- include "common.labels" . }}
Or: {{- include "common.labels" (dict "Release" .Release "Chart" .Chart "Values" .Values) }}
*/}}
{{- define "common.labels" -}}
helm.sh/chart: {{ include "common.chart" . }}
{{ include "common.selectorLabels" . }}
{{- if .Chart }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
Usage: {{- include "common.selectorLabels" . }}
Or: {{- include "common.selectorLabels" (dict "Release" .Release "Chart" .Chart "Values" .Values) }}
*/}}
{{- define "common.selectorLabels" -}}
app.kubernetes.io/name: {{ include "common.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

