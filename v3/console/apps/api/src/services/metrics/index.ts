export { ingestMetric, ingestMetricBatch, queryTimeSeries, queryAggregate, queryLatest } from './metric.service.js';
export { enqueueMetric, startAggregationWorker, stopAggregationWorker } from './aggregation.worker.js';
export { subscribeToInstance, unsubscribeFromInstance, unsubscribeAll, publishMetricUpdate } from './stream.js';
export type {
  IngestMetricInput,
  TimeSeriesFilter,
  AggregateFilter,
  LatestFilter,
  TimeSeriesPoint,
  AggregateResult,
  LatestMetric,
  Granularity,
  MetricName,
} from './types.js';
