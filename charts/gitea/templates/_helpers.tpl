{{/*
Expand the name of the chart.
*/}}
{{- define "gitea.name" -}}
{{- include "common.name" . }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "gitea.fullname" -}}
{{- include "common.fullname" . }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "gitea.chart" -}}
{{- include "common.chart" . }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "gitea.labels" -}}
{{- include "common.labels" . }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "gitea.selectorLabels" -}}
{{- include "common.selectorLabels" . }}
{{- end }}

