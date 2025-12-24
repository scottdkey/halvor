{{/*
Common ingress template for Traefik
Usage: {{- include "common.ingress" (dict "Values" .Values "Release" .Release "Chart" .Chart "name" "app-name" "domain" "app.example.com" "servicePort" 8080) }}
*/}}
{{- define "common.ingress" -}}
{{- if .Values.ingress.enabled }}
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: {{ include "common.fullname" (dict "Values" .Values "Release" .Release "Chart" .Chart "name" .name) }}
  labels:
    {{- include "common.labels" (dict "Values" .Values "Release" .Release "Chart" .Chart "name" .name) | nindent 4 }}
  annotations:
    {{- with .Values.ingress.annotations }}
    {{- toYaml . | nindent 4 }}
    {{- end }}
spec:
  ingressClassName: {{ .Values.ingress.className | default "traefik" }}
  rules:
  - host: {{ .domain | quote }}
    http:
      paths:
      - path: {{ .Values.ingress.path | default "/" }}
        pathType: {{ .Values.ingress.pathType | default "Prefix" }}
        backend:
          service:
            name: {{ include "common.fullname" (dict "Values" .Values "Release" .Release "Chart" .Chart "name" .name) }}
            port:
              number: {{ .servicePort | default .Values.service.port }}
{{- end }}
{{- end }}

