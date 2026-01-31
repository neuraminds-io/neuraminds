import { Metadata } from 'next';
import { ApiDocumentation } from '@/components/docs/ApiDocumentation';

export const metadata: Metadata = {
  title: 'API Documentation | PolyBit',
  description: 'REST API documentation for PolyBit prediction market platform',
};

export default function ApiDocsPage() {
  return <ApiDocumentation />;
}
