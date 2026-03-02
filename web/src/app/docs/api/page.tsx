import { Metadata } from 'next';
import { ApiDocumentation } from '@/components/docs/ApiDocumentation';

export const metadata: Metadata = {
  title: 'API Documentation | neuraminds',
  description: 'REST API documentation for neuraminds prediction market platform',
};

export default function ApiDocsPage() {
  return <ApiDocumentation />;
}
