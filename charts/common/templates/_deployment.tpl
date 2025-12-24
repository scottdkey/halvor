{{/*
Common deployment template
Usage: {{- include "common.deployment" (dict "Values" .Values "Release" .Release "Chart" .Chart "name" "app-name" "image" "image:tag" "port" 8080) }}
*/}}
{{- define "common.deployment" -}}
apiVersion: apps/v1
kind: Deployment
metadata:
  name: {{ include "common.fullname" (dict "Values" .Values "Release" .Release "Chart" .Chart "name" .name) }}
  labels:
    {{- include "common.labels" (dict "Values" .Values "Release" .Release "Chart" .Chart "name" .name) | nindent 4 }}
spec:
  replicas: {{ .Values.replicas | default 1 }}
  selector:
    matchLabels:
      {{- include "common.selectorLabels" (dict "Values" .Values "Release" .Release "Chart" .Chart "name" .name) | nindent 6 }}
  template:
    metadata:
      labels:
        {{- include "common.selectorLabels" (dict "Values" .Values "Release" .Release "Chart" .Chart "name" .name) | nindent 8 }}
    spec:
      {{- if .Values.securityContext }}
      securityContext:
        {{- toYaml .Values.securityContext | nindent 8 }}
      {{- end }}
      containers:
      - name: {{ .name }}
        image: "{{ .image }}"
        imagePullPolicy: {{ .Values.image.pullPolicy | default "IfNotPresent" }}
        {{- if .port }}
        ports:
        - name: http
          containerPort: {{ .port }}
          protocol: TCP
        {{- end }}
        {{- if .env }}
        env:
        {{- range $key, $value := .env }}
        - name: {{ $key }}
          value: {{ $value | quote }}
        {{- end }}
        {{- end }}
        {{- if .volumeMounts }}
        volumeMounts:
        {{- toYaml .volumeMounts | nindent 8 }}
        {{- end }}
        {{- if .Values.resources }}
        resources:
          {{- toYaml .Values.resources | nindent 10 }}
        {{- end }}
        {{- if .livenessProbe }}
        livenessProbe:
          {{- toYaml .livenessProbe | nindent 10 }}
        {{- end }}
        {{- if .readinessProbe }}
        readinessProbe:
          {{- toYaml .readinessProbe | nindent 10 }}
        {{- end }}
      {{- if .volumes }}
      volumes:
      {{- toYaml .volumes | nindent 6 }}
      {{- end }}
      {{- with .Values.nodeSelector }}
      nodeSelector:
        {{- toYaml . | nindent 8 }}
      {{- end }}
      {{- with .Values.affinity }}
      affinity:
        {{- toYaml . | nindent 8 }}
      {{- end }}
      {{- with .Values.tolerations }}
      tolerations:
        {{- toYaml . | nindent 8 }}
      {{- end }}
{{- end }}

