{{/*
Expand the name of the chart.
*/}}
{{- define "traefik.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "traefik.fullname" -}}
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
{{- define "traefik.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "traefik.labels" -}}
helm.sh/chart: {{ include "traefik.chart" . }}
{{ include "traefik.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
app.kubernetes.io/component: ingress-controller
traefik.io/mode: {{ .Values.mode }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "traefik.selectorLabels" -}}
app.kubernetes.io/name: {{ include "traefik.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
Ingress class name - auto-generated based on mode if not specified
*/}}
{{- define "traefik.ingressClassName" -}}
{{- if .Values.ingressClass.name }}
{{- .Values.ingressClass.name }}
{{- else }}
{{- printf "traefik-%s" .Values.mode }}
{{- end }}
{{- end }}

{{/*
Is default ingress class - auto-set based on mode if not specified
*/}}
{{- define "traefik.isDefaultIngressClass" -}}
{{- if not (kindIs "invalid" .Values.ingressClass.isDefault) }}
{{- .Values.ingressClass.isDefault | toString }}
{{- else if eq .Values.mode "private" }}
{{- "true" }}
{{- else }}
{{- "false" }}
{{- end }}
{{- end }}

{{/*
Node hostname - auto-set based on mode if not specified
*/}}
{{- define "traefik.nodeHostname" -}}
{{- if .Values.node.hostname }}
{{- .Values.node.hostname }}
{{- else if eq .Values.mode "public" }}
{{- "oak" }}
{{- else }}
{{- "frigg" }}
{{- end }}
{{- end }}

{{/*
Controller identifier for the ingress class
*/}}
{{- define "traefik.controllerName" -}}
{{- printf "traefik.io/ingress-controller-%s" .Values.mode }}
{{- end }}
