use chrono::Utc;
use opentelemetry::trace::TraceContextExt;
use opentelemetry::{SpanId, TraceId};
use serde::ser::{SerializeMap, Serializer as _};
use std::io;
use tracing::{Event, Subscriber};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use tracing_serde::fields::AsMap;
use tracing_serde::AsSerde;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

pub struct WriteAdaptor<'a> {
    fmt_write: &'a mut dyn std::fmt::Write,
}

impl<'a> WriteAdaptor<'a> {
    pub fn new(fmt_write: &'a mut dyn std::fmt::Write) -> Self {
        Self { fmt_write }
    }
}

impl<'a> io::Write for WriteAdaptor<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s =
            std::str::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        self.fmt_write
            .write_str(s)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(s.as_bytes().len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub struct TraceIdFormat;

impl<S, N> FormatEvent<S, N> for TraceIdFormat
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        let meta = event.metadata();

        let mut visit = || {
            let mut serializer = serde_json::Serializer::new(WriteAdaptor::new(&mut writer));
            let mut serializer = serializer.serialize_map(None)?;
            serializer.serialize_entry("timestamp", &Utc::now().to_rfc3339())?;
            serializer.serialize_entry("level", &meta.level().as_serde())?;
            serializer.serialize_entry("fields", &event.field_map())?;
            serializer.serialize_entry("target", meta.target())?;

            let current_span = tracing::Span::current();
            let context = current_span.context();
            let span_ref = context.span();
            let span_context = span_ref.span_context();

            let trace_id = span_context.trace_id();
            if trace_id != TraceId::INVALID {
                serializer.serialize_entry("trace_id", &trace_id.to_string())?;

                let span_id = span_context.span_id();
                if span_id != SpanId::INVALID {
                    serializer.serialize_entry("span_id", &span_id.to_string())?;
                }
            }

            serializer.end()
        };

        visit().map_err(|_| std::fmt::Error)?;
        writeln!(writer)
    }
}
