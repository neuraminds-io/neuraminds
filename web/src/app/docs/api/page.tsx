import { Metadata } from 'next';
import { ApiDocumentation } from '@/components/docs/ApiDocumentation';

export const metadata: Metadata = {
  title: 'API Documentation | Polyguard',
  description: 'REST API documentation for Polyguard prediction market platform',
};

export default function ApiDocsPage() {
  return <ApiDocumentation />;
}
