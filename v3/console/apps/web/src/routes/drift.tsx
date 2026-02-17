import { createFileRoute } from '@tanstack/react-router';
import { DriftPage } from '@/pages/DriftPage';

export const Route = createFileRoute('/drift')({
  component: DriftPage,
});
