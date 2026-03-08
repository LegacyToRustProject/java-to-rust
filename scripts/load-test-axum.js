// k6 load test script for Axum greeting service
// Usage: k6 run --vus 50 --duration 60s scripts/load-test-axum.js
import http from 'k6/http';
import { check, sleep } from 'k6';
import { Rate } from 'k6/metrics';

const errorRate = new Rate('errors');

export const options = {
  vus: 50,
  duration: '60s',
  thresholds: {
    http_req_duration: ['p(99)<100'], // 99th percentile < 100ms
    errors: ['rate<0.01'],            // Error rate < 1%
  },
};

export default function () {
  const res = http.get('http://localhost:8080/greeting?name=k6');
  const ok = check(res, {
    'status is 200': (r) => r.status === 200,
    'response has content': (r) => r.body.includes('Hello'),
    'response time < 50ms': (r) => r.timings.duration < 50,
  });
  errorRate.add(!ok);
  sleep(0.01);
}
