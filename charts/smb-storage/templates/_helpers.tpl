{{/*
Expand the name of the chart.
*/}}
{{- define "smb-storage.name" -}}
{{- include "common.name" . }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "smb-storage.fullname" -}}
{{- include "common.fullname" . }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "smb-storage.chart" -}}
{{- include "common.chart" . }}
{{- end }}

{{/*
Common labels
*/}}
{{- define "smb-storage.labels" -}}
{{- include "common.labels" . }}
{{- end }}

{{/*
Selector labels
*/}}
{{- define "smb-storage.selectorLabels" -}}
{{- include "common.selectorLabels" . }}
{{- end }}

