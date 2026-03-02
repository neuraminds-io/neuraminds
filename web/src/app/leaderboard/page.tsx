import { Metadata } from 'next';
import { LeaderboardTable } from '@/components/leaderboard';

export const metadata: Metadata = {
  title: 'Leaderboard | neuraminds',
  description: 'Top traders on the neuraminds prediction market',
};

export default function LeaderboardPage() {
  return (
    <div className="container mx-auto px-4 py-8">
      <LeaderboardTable limit={100} />
    </div>
  );
}
