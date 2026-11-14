{{/* Chart name, truncated to the 63 characters Kubernetes allows. */}}
{{- define "t-rex.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/* Release-scoped name: <release>-<chart>, unless the release name already contains the chart name. */}}
{{- define "t-rex.fullname" -}}
{{- if contains .Chart.Name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name .Chart.Name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}

{{- define "t-rex.labels" -}}
helm.sh/chart: {{ printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{ include "t-rex.selectorLabels" . }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{- define "t-rex.selectorLabels" -}}
app.kubernetes.io/name: {{ include "t-rex.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{- define "t-rex.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "t-rex.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/* Name of the secret with the OIDC settings: the existing one, or the one this chart creates. */}}
{{- define "t-rex.oidcSecretName" -}}
{{- default (printf "%s-oidc" (include "t-rex.fullname" .)) .Values.oidc.existingSecret }}
{{- end }}

{{/* Name of the secret with the Postgres password. */}}
{{- define "t-rex.dbSecretName" -}}
{{- default (printf "%s-db" (include "t-rex.fullname" .)) .Values.database.existingSecret }}
{{- end }}

{{/* True when the app uses Postgres (a database URL is set) instead of the SQLite volume. */}}
{{- define "t-rex.usesPostgres" -}}
{{- if .Values.database.url }}true{{ end }}
{{- end }}
