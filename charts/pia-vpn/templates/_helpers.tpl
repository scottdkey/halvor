{{/*
Expand the name of the chart.
*/}}
{{- define "pia-vpn.name" -}}
{{- if .Chart }}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- default "pia-vpn" .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "pia-vpn.fullname" -}}
{{- if .Release -}}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else -}}
pia-vpn
{{- end -}}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "pia-vpn.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "pia-vpn.labels" -}}
helm.sh/chart: {{ include "pia-vpn.chart" . }}
{{ include "pia-vpn.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "pia-vpn.selectorLabels" -}}
app.kubernetes.io/name: {{ include "pia-vpn.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

