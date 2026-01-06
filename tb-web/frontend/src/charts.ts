/**
 * Chart.js configuration for balance history visualization.
 */

import { formatTimestamp } from './formatters';

declare const Chart: any;

interface AccountBalance {
    debits_pending: string;
    debits_posted: string;
    credits_pending: string;
    credits_posted: string;
    timestamp: number;
}

interface BalancesResponse {
    balances: AccountBalance[];
}

let balanceChart: any = null;

/**
 * Initialize and render the balance history chart for an account.
 */
export async function renderBalanceChart(
    accountId: string,
    canvasId: string = 'balanceChart'
): Promise<void> {
    const canvas = document.getElementById(canvasId) as HTMLCanvasElement;
    if (!canvas) {
        console.error(`Canvas element '${canvasId}' not found`);
        return;
    }

    try {
        const response = await fetch(`/api/v1/accounts/${accountId}/balances?limit=1000`);
        if (!response.ok) {
            throw new Error(`Failed to fetch balances: ${response.statusText}`);
        }

        const data: BalancesResponse = await response.json();

        if (data.balances.length === 0) {
            canvas.parentElement!.innerHTML = '<p class="loading">No balance history available. Account may not have HISTORY flag enabled.</p>';
            return;
        }

        // Sort by timestamp (oldest first for chronological display)
        const sortedBalances = [...data.balances].sort((a, b) => a.timestamp - b.timestamp);

        // Prepare chart data
        const labels = sortedBalances.map(b => formatTimestamp(b.timestamp));
        const netBalances = sortedBalances.map(b => {
            const credits = BigInt(b.credits_posted);
            const debits = BigInt(b.debits_posted);
            // Convert to number for Chart.js (may lose precision for very large numbers)
            return Number(credits - debits);
        });
        const creditsData = sortedBalances.map(b => Number(BigInt(b.credits_posted)));
        const debitsData = sortedBalances.map(b => Number(BigInt(b.debits_posted)));

        // Destroy existing chart if any
        if (balanceChart) {
            balanceChart.destroy();
        }

        const ctx = canvas.getContext('2d')!;
        balanceChart = new Chart(ctx, {
            type: 'line',
            data: {
                labels,
                datasets: [
                    {
                        label: 'Net Balance',
                        data: netBalances,
                        borderColor: '#f7931a',
                        backgroundColor: 'rgba(247, 147, 26, 0.1)',
                        fill: true,
                        tension: 0.1,
                    },
                    {
                        label: 'Credits Posted',
                        data: creditsData,
                        borderColor: '#00ba7c',
                        backgroundColor: 'transparent',
                        borderDash: [5, 5],
                        tension: 0.1,
                    },
                    {
                        label: 'Debits Posted',
                        data: debitsData,
                        borderColor: '#f4212e',
                        backgroundColor: 'transparent',
                        borderDash: [5, 5],
                        tension: 0.1,
                    },
                ],
            },
            options: {
                responsive: true,
                maintainAspectRatio: false,
                plugins: {
                    legend: {
                        labels: {
                            color: '#e7e9ea',
                        },
                    },
                    tooltip: {
                        mode: 'index',
                        intersect: false,
                    },
                },
                scales: {
                    x: {
                        ticks: {
                            color: '#8b98a5',
                            maxRotation: 45,
                            minRotation: 45,
                        },
                        grid: {
                            color: '#2f3336',
                        },
                    },
                    y: {
                        ticks: {
                            color: '#8b98a5',
                            callback: function(value: number) {
                                return value.toLocaleString();
                            },
                        },
                        grid: {
                            color: '#2f3336',
                        },
                    },
                },
                interaction: {
                    mode: 'nearest',
                    axis: 'x',
                    intersect: false,
                },
            },
        });
    } catch (error) {
        console.error('Error rendering balance chart:', error);
        canvas.parentElement!.innerHTML = `<p class="loading">Error loading balance history: ${error}</p>`;
    }
}

/**
 * Destroy the current chart instance.
 */
export function destroyChart(): void {
    if (balanceChart) {
        balanceChart.destroy();
        balanceChart = null;
    }
}
