-- Retry policy plugin, loaded by the scheduler at startup
local M = {}

function M.should_retry(job, attempt)
  if job.status ~= "failed" then
    return false
  end
  return attempt < 3 and job.exit_code ~= 137
end

function M.backoff(attempt)
  return math.min(2 ^ attempt, 60)
end

return M
