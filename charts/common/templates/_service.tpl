{{/*
Common service template
Usage: {{- include "common.service" (dict "Values" .Values "Release" .Release "Chart" .Chart "name" "app-name" "port" 8080) }}
*/}}
{{- define "common.service" -}}
apiVersion: v1
kind: Service
metadata:
  name: {{ include "common.fullname" (dict "Values" .Values "Release" .Release "Chart" .Chart "name" .name) }}
  labels:
    {{- include "common.labels" (dict "Values" .Values "Release" .Release "Chart" .Chart "name" .name) | nindent 4 }}
spec:
  type: {{ .Values.service.type | default "ClusterIP" }}
  ports:
  - port: {{ .Values.service.port | default .port }}
    targetPort: http
    protocol: TCP
    name: http
  selector:
    {{- include "common.selectorLabels" (dict "Values" .Values "Release" .Release "Chart" .Chart "name" .name) | nindent 4 }}
{{- end }}

