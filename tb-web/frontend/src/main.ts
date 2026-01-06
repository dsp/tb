/**
 * TigerBeetle Web - Main entry point.
 */

import {
    formatId,
    formatAmount,
    formatTimestamp,
    formatAccountFlags,
    formatTransferFlags,
    calculateNetBalance,
    formatBalance,
} from './formatters';
import { renderBalanceChart, destroyChart } from './charts';

// Make functions available globally for HTMX integration
declare global {
    interface Window {
        tbWeb: {
            formatId: typeof formatId;
            formatAmount: typeof formatAmount;
            formatTimestamp: typeof formatTimestamp;
            formatAccountFlags: typeof formatAccountFlags;
            formatTransferFlags: typeof formatTransferFlags;
            calculateNetBalance: typeof calculateNetBalance;
            formatBalance: typeof formatBalance;
            renderBalanceChart: typeof renderBalanceChart;
            destroyChart: typeof destroyChart;
            renderAccountsTable: typeof renderAccountsTable;
            renderTransfersTable: typeof renderTransfersTable;
        };
    }
}

interface ApiAccount {
    id: string;
    debits_pending: string;
    debits_posted: string;
    credits_pending: string;
    credits_posted: string;
    user_data_128: string;
    user_data_64: number;
    user_data_32: number;
    ledger: number;
    code: number;
    flags: number;
    timestamp: number;
}

interface ApiTransfer {
    id: string;
    debit_account_id: string;
    credit_account_id: string;
    amount: string;
    pending_id: string;
    user_data_128: string;
    user_data_64: number;
    user_data_32: number;
    timeout: number;
    ledger: number;
    code: number;
    flags: number;
    timestamp: number;
}

interface AccountsResponse {
    accounts: ApiAccount[];
    next_timestamp?: number;
}

interface TransfersResponse {
    transfers: ApiTransfer[];
    next_timestamp?: number;
}

/**
 * Render accounts table from API response.
 */
function renderAccountsTable(containerId: string, data: AccountsResponse): void {
    const container = document.getElementById(containerId);
    if (!container) return;

    if (data.accounts.length === 0) {
        container.innerHTML = '<p class="loading">No accounts found</p>';
        return;
    }

    const html = `
        <table>
            <thead>
                <tr>
                    <th>ID</th>
                    <th>Ledger</th>
                    <th>Code</th>
                    <th class="amount">Net Balance</th>
                    <th class="amount">Credits</th>
                    <th class="amount">Debits</th>
                    <th>Created</th>
                </tr>
            </thead>
            <tbody>
                ${data.accounts.map(account => {
                    const netBalance = calculateNetBalance(account.credits_posted, account.debits_posted);
                    const balanceClass = netBalance >= 0n ? 'positive' : 'negative';
                    return `
                        <tr>
                            <td><a href="/account/${account.id}" class="id" title="${account.id}">${formatId(account.id)}</a></td>
                            <td>${account.ledger}</td>
                            <td>${account.code}</td>
                            <td class="amount ${balanceClass}">${formatBalance(netBalance)}</td>
                            <td class="amount">${formatAmount(account.credits_posted)}</td>
                            <td class="amount">${formatAmount(account.debits_posted)}</td>
                            <td>${formatTimestamp(account.timestamp)}</td>
                        </tr>
                    `;
                }).join('')}
            </tbody>
        </table>
        ${data.next_timestamp ? `
            <div class="pagination">
                <button class="btn btn-secondary"
                        hx-get="/api/v1/accounts?limit=100&after_timestamp=${data.next_timestamp}"
                        hx-target="#${containerId}">
                    Load More
                </button>
            </div>
        ` : ''}
    `;

    container.innerHTML = html;
    // Re-process HTMX attributes
    (window as any).htmx?.process(container);
}

/**
 * Render transfers table from API response.
 */
function renderTransfersTable(containerId: string, data: TransfersResponse): void {
    const container = document.getElementById(containerId);
    if (!container) return;

    if (data.transfers.length === 0) {
        container.innerHTML = '<p class="loading">No transfers found</p>';
        return;
    }

    const html = `
        <table>
            <thead>
                <tr>
                    <th>ID</th>
                    <th>From</th>
                    <th>To</th>
                    <th class="amount">Amount</th>
                    <th>Ledger</th>
                    <th>Code</th>
                    <th>Created</th>
                </tr>
            </thead>
            <tbody>
                ${data.transfers.map(transfer => `
                    <tr>
                        <td><a href="/transfer/${transfer.id}" class="id" title="${transfer.id}">${formatId(transfer.id)}</a></td>
                        <td><a href="/account/${transfer.debit_account_id}" class="id" title="${transfer.debit_account_id}">${formatId(transfer.debit_account_id)}</a></td>
                        <td><a href="/account/${transfer.credit_account_id}" class="id" title="${transfer.credit_account_id}">${formatId(transfer.credit_account_id)}</a></td>
                        <td class="amount">${formatAmount(transfer.amount)}</td>
                        <td>${transfer.ledger}</td>
                        <td>${transfer.code}</td>
                        <td>${formatTimestamp(transfer.timestamp)}</td>
                    </tr>
                `).join('')}
            </tbody>
        </table>
        ${data.next_timestamp ? `
            <div class="pagination">
                <button class="btn btn-secondary"
                        hx-get="/api/v1/transfers?limit=100&reversed=true&after_timestamp=${data.next_timestamp}"
                        hx-target="#${containerId}">
                    Load More
                </button>
            </div>
        ` : ''}
    `;

    container.innerHTML = html;
    // Re-process HTMX attributes
    (window as any).htmx?.process(container);
}

// Initialize
document.addEventListener('DOMContentLoaded', () => {
    console.log('TigerBeetle Web initialized');

    // Expose functions globally
    window.tbWeb = {
        formatId,
        formatAmount,
        formatTimestamp,
        formatAccountFlags,
        formatTransferFlags,
        calculateNetBalance,
        formatBalance,
        renderBalanceChart,
        destroyChart,
        renderAccountsTable,
        renderTransfersTable,
    };

    // Handle HTMX events to transform JSON responses into HTML
    document.body.addEventListener('htmx:beforeSwap', (event: any) => {
        const target = event.detail.target;
        const xhr = event.detail.xhr;

        // Check if this is an API response
        if (xhr.getResponseHeader('content-type')?.includes('application/json')) {
            try {
                const data = JSON.parse(xhr.responseText);

                // Determine what kind of data this is and render appropriately
                if (data.accounts) {
                    event.detail.shouldSwap = false;
                    renderAccountsTable(target.id, data);
                } else if (data.transfers) {
                    event.detail.shouldSwap = false;
                    renderTransfersTable(target.id, data);
                }
            } catch (e) {
                console.error('Error parsing JSON response:', e);
            }
        }
    });
});
