{{/*
Common PVC template
Usage: {{- include "common.pvc" (dict "Values" .Values "Release" .Release "Chart" .Chart "name" "app-name" "size" "10Gi" "storageClass" "smb-storage") }}
*/}}
{{- define "common.pvc" -}}
{{- if .Values.persistence.enabled }}
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: {{ include "common.fullname" (dict "Values" .Values "Release" .Release "Chart" .Chart "name" .name) }}-data
  labels:
    {{- include "common.labels" (dict "Values" .Values "Release" .Release "Chart" .Chart "name" .name) | nindent 4 }}
spec:
  accessModes:
    - {{ .Values.persistence.accessMode | default "ReadWriteOnce" }}
  {{- if .storageClass }}
  storageClassName: {{ .storageClass }}
  {{- else if .Values.persistence.storageClass }}
  storageClassName: {{ .Values.persistence.storageClass }}
  {{- end }}
  resources:
    requests:
      storage: {{ .size | default .Values.persistence.size | default "10Gi" }}
{{- end }}
{{- end }}

