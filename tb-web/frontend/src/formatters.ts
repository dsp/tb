/**
 * Formatting utilities for TigerBeetle data types.
 */

/**
 * Format a u128 ID (hex string) to a shortened display format.
 */
export function formatId(id: string): string {
    if (!id || id === '00000000000000000000000000000000') {
        return '-';
    }
    // Show first 8 and last 8 characters
    if (id.length > 16) {
        return `${id.slice(0, 8)}...${id.slice(-8)}`;
    }
    return id;
}

/**
 * Format a u128 ID for full display.
 */
export function formatIdFull(id: string): string {
    if (!id || id === '00000000000000000000000000000000') {
        return '-';
    }
    return id;
}

/**
 * Format a large number string with thousands separators.
 */
export function formatAmount(amount: string): string {
    if (!amount || amount === '0') {
        return '0';
    }
    // Add thousands separators
    return amount.replace(/\B(?=(\d{3})+(?!\d))/g, ',');
}

/**
 * Format a TigerBeetle timestamp (nanoseconds since epoch) to a readable date.
 */
export function formatTimestamp(timestamp: number): string {
    if (!timestamp) {
        return '-';
    }
    // TigerBeetle timestamps are in nanoseconds
    const ms = timestamp / 1_000_000;
    const date = new Date(ms);
    return date.toLocaleString();
}

/**
 * Format a TigerBeetle timestamp to ISO format.
 */
export function formatTimestampIso(timestamp: number): string {
    if (!timestamp) {
        return '-';
    }
    const ms = timestamp / 1_000_000;
    const date = new Date(ms);
    return date.toISOString();
}

/**
 * Format account flags to a human-readable string.
 */
export function formatAccountFlags(flags: number): string {
    const flagNames: string[] = [];

    if (flags & (1 << 0)) flagNames.push('LINKED');
    if (flags & (1 << 1)) flagNames.push('DEBITS_MUST_NOT_EXCEED_CREDITS');
    if (flags & (1 << 2)) flagNames.push('CREDITS_MUST_NOT_EXCEED_DEBITS');
    if (flags & (1 << 3)) flagNames.push('HISTORY');
    if (flags & (1 << 4)) flagNames.push('IMPORTED');
    if (flags & (1 << 5)) flagNames.push('CLOSED');

    return flagNames.length > 0 ? flagNames.join(', ') : 'none';
}

/**
 * Format transfer flags to a human-readable string.
 */
export function formatTransferFlags(flags: number): string {
    const flagNames: string[] = [];

    if (flags & (1 << 0)) flagNames.push('LINKED');
    if (flags & (1 << 1)) flagNames.push('PENDING');
    if (flags & (1 << 2)) flagNames.push('POST_PENDING');
    if (flags & (1 << 3)) flagNames.push('VOID_PENDING');
    if (flags & (1 << 4)) flagNames.push('BALANCING_DEBIT');
    if (flags & (1 << 5)) flagNames.push('BALANCING_CREDIT');
    if (flags & (1 << 6)) flagNames.push('CLOSING_DEBIT');
    if (flags & (1 << 7)) flagNames.push('CLOSING_CREDIT');
    if (flags & (1 << 8)) flagNames.push('IMPORTED');

    return flagNames.length > 0 ? flagNames.join(', ') : 'none';
}

/**
 * Calculate net balance from an account.
 */
export function calculateNetBalance(
    creditsPosted: string,
    debitsPosted: string
): bigint {
    return BigInt(creditsPosted) - BigInt(debitsPosted);
}

/**
 * Format a BigInt balance with sign.
 */
export function formatBalance(balance: bigint): string {
    const formatted = formatAmount(balance.toString().replace('-', ''));
    if (balance < 0n) {
        return `-${formatted}`;
    }
    return formatted;
}
