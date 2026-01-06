//! HTML template rendering for HTMX responses.

use crate::api::{ApiAccount, ApiTransfer};

/// Format a u128 hex ID for display (shortened).
fn format_id(id: &str) -> String {
    if id == "00000000000000000000000000000000" {
        return "-".to_string();
    }
    if id.len() > 16 {
        format!("{}...{}", &id[..8], &id[id.len() - 8..])
    } else {
        id.to_string()
    }
}

/// Format a number string with thousands separators.
fn format_amount(amount: &str) -> String {
    if amount == "0" {
        return "0".to_string();
    }
    let mut result = String::new();
    let chars: Vec<char> = amount.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }
    result
}

/// Format a TigerBeetle timestamp (nanoseconds) to a readable date.
fn format_timestamp(timestamp: u64) -> String {
    if timestamp == 0 {
        return "-".to_string();
    }
    // Convert nanoseconds to milliseconds
    let ms = timestamp / 1_000_000;
    let secs = (ms / 1000) as i64;
    let nanos = ((ms % 1000) * 1_000_000) as u32;

    // Use chrono-free formatting (simple approach)
    let datetime = std::time::UNIX_EPOCH + std::time::Duration::new(secs as u64, nanos);
    let duration = datetime
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = duration.as_secs();

    // Simple date formatting without chrono
    let days = total_secs / 86400;
    let years = 1970 + days / 365; // Approximate
    let remaining_days = days % 365;
    let months = remaining_days / 30 + 1;
    let day = remaining_days % 30 + 1;
    let hours = (total_secs % 86400) / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        years, months, day, hours, minutes, seconds
    )
}

/// Calculate net balance from credits and debits.
fn calculate_net_balance(credits_posted: &str, debits_posted: &str) -> (String, bool) {
    let credits: i128 = credits_posted.parse().unwrap_or(0);
    let debits: i128 = debits_posted.parse().unwrap_or(0);
    let net = credits - debits;
    let is_positive = net >= 0;
    let formatted = format_amount(&net.abs().to_string());
    if net < 0 {
        (format!("-{}", formatted), is_positive)
    } else {
        (formatted, is_positive)
    }
}

/// Render accounts as an HTML table.
pub fn render_accounts_table(accounts: &[ApiAccount], next_timestamp: Option<u64>) -> String {
    if accounts.is_empty() {
        return r#"<p class="loading">No accounts found</p>"#.to_string();
    }

    let mut html = String::from(
        r#"<table>
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
            <tbody>"#,
    );

    for account in accounts {
        let (net_balance, is_positive) = calculate_net_balance(&account.credits_posted, &account.debits_posted);
        let balance_class = if is_positive { "positive" } else { "negative" };

        html.push_str(&format!(
            r#"<tr>
                <td><a href="/account/{}" class="id" title="{}">{}</a></td>
                <td>{}</td>
                <td>{}</td>
                <td class="amount {}">{}</td>
                <td class="amount">{}</td>
                <td class="amount">{}</td>
                <td>{}</td>
            </tr>"#,
            account.id,
            account.id,
            format_id(&account.id),
            account.ledger,
            account.code,
            balance_class,
            net_balance,
            format_amount(&account.credits_posted),
            format_amount(&account.debits_posted),
            format_timestamp(account.timestamp),
        ));
    }

    html.push_str("</tbody></table>");

    if let Some(ts) = next_timestamp {
        html.push_str(&format!(
            r#"<div class="pagination">
                <button class="btn btn-secondary"
                        hx-get="/api/v1/accounts?limit=100&after_timestamp={}"
                        hx-target="closest .recent-section div"
                        hx-swap="innerHTML">
                    Load More
                </button>
            </div>"#,
            ts
        ));
    }

    html
}

/// Render transfers as an HTML table.
pub fn render_transfers_table(transfers: &[ApiTransfer], next_timestamp: Option<u64>) -> String {
    if transfers.is_empty() {
        return r#"<p class="loading">No transfers found</p>"#.to_string();
    }

    let mut html = String::from(
        r#"<table>
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
            <tbody>"#,
    );

    for transfer in transfers {
        html.push_str(&format!(
            r#"<tr>
                <td><a href="/transfer/{}" class="id" title="{}">{}</a></td>
                <td><a href="/account/{}" class="id" title="{}">{}</a></td>
                <td><a href="/account/{}" class="id" title="{}">{}</a></td>
                <td class="amount">{}</td>
                <td>{}</td>
                <td>{}</td>
                <td>{}</td>
            </tr>"#,
            transfer.id,
            transfer.id,
            format_id(&transfer.id),
            transfer.debit_account_id,
            transfer.debit_account_id,
            format_id(&transfer.debit_account_id),
            transfer.credit_account_id,
            transfer.credit_account_id,
            format_id(&transfer.credit_account_id),
            format_amount(&transfer.amount),
            transfer.ledger,
            transfer.code,
            format_timestamp(transfer.timestamp),
        ));
    }

    html.push_str("</tbody></table>");

    if let Some(ts) = next_timestamp {
        html.push_str(&format!(
            r#"<div class="pagination">
                <button class="btn btn-secondary"
                        hx-get="/api/v1/transfers?limit=100&reversed=true&after_timestamp={}"
                        hx-target="closest .recent-section div"
                        hx-swap="innerHTML">
                    Load More
                </button>
            </div>"#,
            ts
        ));
    }

    html
}

/// Render a stat card for accounts count.
pub fn render_accounts_stat(accounts: &[ApiAccount]) -> String {
    format!(
        r#"<div class="label">Accounts</div>
        <div class="value">{}</div>"#,
        accounts.len()
    )
}

/// Render a stat card for transfers count.
pub fn render_transfers_stat(transfers: &[ApiTransfer]) -> String {
    format!(
        r#"<div class="label">Recent Transfers</div>
        <div class="value">{}</div>"#,
        transfers.len()
    )
}

/// Render account detail page.
pub fn render_account_detail(account: &ApiAccount) -> String {
    let (net_balance, is_positive) = calculate_net_balance(&account.credits_posted, &account.debits_posted);
    let balance_class = if is_positive { "positive" } else { "negative" };

    format!(
        r#"<section class="account-detail-page">
            <h2>Account Details</h2>

            <div class="account-detail">
                <div class="account-info">
                    <h3>Information</h3>
                    <div class="info-row">
                        <span class="info-label">ID</span>
                        <span class="info-value">{}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">Ledger</span>
                        <span class="info-value">{}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">Code</span>
                        <span class="info-value">{}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">Flags</span>
                        <span class="info-value">{}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">Created</span>
                        <span class="info-value">{}</span>
                    </div>
                </div>

                <div class="account-info">
                    <h3>Balances</h3>
                    <div class="info-row">
                        <span class="info-label">Net Balance</span>
                        <span class="info-value {}">{}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">Credits Posted</span>
                        <span class="info-value">{}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">Debits Posted</span>
                        <span class="info-value">{}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">Credits Pending</span>
                        <span class="info-value">{}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">Debits Pending</span>
                        <span class="info-value">{}</span>
                    </div>
                </div>
            </div>

            <div class="chart-container">
                <h3>Balance History</h3>
                <canvas id="balanceChart" height="300"></canvas>
            </div>
            <script>
                if (window.tbWeb && window.tbWeb.renderBalanceChart) {{
                    window.tbWeb.renderBalanceChart('{}');
                }}
            </script>

            <div class="recent-section">
                <h3>Recent Transfers</h3>
                <div id="account-transfers"
                     hx-get="/api/v1/accounts/{}/transfers?limit=20&reversed=true"
                     hx-trigger="load">
                    Loading transfers...
                </div>
            </div>
        </section>"#,
        account.id,
        account.ledger,
        account.code,
        format_account_flags(account.flags),
        format_timestamp(account.timestamp),
        balance_class,
        net_balance,
        format_amount(&account.credits_posted),
        format_amount(&account.debits_posted),
        format_amount(&account.credits_pending),
        format_amount(&account.debits_pending),
        account.id,
        account.id,
    )
}

/// Render transfer detail page.
pub fn render_transfer_detail(transfer: &ApiTransfer) -> String {
    format!(
        r#"<section class="transfer-detail-page">
            <h2>Transfer Details</h2>

            <div class="account-detail">
                <div class="account-info">
                    <h3>Transfer</h3>
                    <div class="info-row">
                        <span class="info-label">ID</span>
                        <span class="info-value">{}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">Amount</span>
                        <span class="info-value">{}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">Ledger</span>
                        <span class="info-value">{}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">Code</span>
                        <span class="info-value">{}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">Flags</span>
                        <span class="info-value">{}</span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">Created</span>
                        <span class="info-value">{}</span>
                    </div>
                </div>

                <div class="account-info">
                    <h3>Accounts</h3>
                    <div class="info-row">
                        <span class="info-label">From (Debit)</span>
                        <span class="info-value"><a href="/account/{}" class="id">{}</a></span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">To (Credit)</span>
                        <span class="info-value"><a href="/account/{}" class="id">{}</a></span>
                    </div>
                    <div class="info-row">
                        <span class="info-label">Pending ID</span>
                        <span class="info-value">{}</span>
                    </div>
                </div>
            </div>
        </section>"#,
        transfer.id,
        format_amount(&transfer.amount),
        transfer.ledger,
        transfer.code,
        format_transfer_flags(transfer.flags),
        format_timestamp(transfer.timestamp),
        transfer.debit_account_id,
        format_id(&transfer.debit_account_id),
        transfer.credit_account_id,
        format_id(&transfer.credit_account_id),
        format_id(&transfer.pending_id),
    )
}

/// Format account flags.
fn format_account_flags(flags: u16) -> String {
    let mut names = Vec::new();
    if flags & (1 << 0) != 0 { names.push("LINKED"); }
    if flags & (1 << 1) != 0 { names.push("DEBITS_MUST_NOT_EXCEED_CREDITS"); }
    if flags & (1 << 2) != 0 { names.push("CREDITS_MUST_NOT_EXCEED_DEBITS"); }
    if flags & (1 << 3) != 0 { names.push("HISTORY"); }
    if flags & (1 << 4) != 0 { names.push("IMPORTED"); }
    if flags & (1 << 5) != 0 { names.push("CLOSED"); }
    if names.is_empty() { "none".to_string() } else { names.join(", ") }
}

/// Format transfer flags.
fn format_transfer_flags(flags: u16) -> String {
    let mut names = Vec::new();
    if flags & (1 << 0) != 0 { names.push("LINKED"); }
    if flags & (1 << 1) != 0 { names.push("PENDING"); }
    if flags & (1 << 2) != 0 { names.push("POST_PENDING"); }
    if flags & (1 << 3) != 0 { names.push("VOID_PENDING"); }
    if flags & (1 << 4) != 0 { names.push("BALANCING_DEBIT"); }
    if flags & (1 << 5) != 0 { names.push("BALANCING_CREDIT"); }
    if flags & (1 << 6) != 0 { names.push("CLOSING_DEBIT"); }
    if flags & (1 << 7) != 0 { names.push("CLOSING_CREDIT"); }
    if flags & (1 << 8) != 0 { names.push("IMPORTED"); }
    if names.is_empty() { "none".to_string() } else { names.join(", ") }
}
