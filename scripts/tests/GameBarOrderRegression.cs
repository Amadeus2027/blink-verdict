namespace BlinkVerdict.GameBar
{
    public static class DisplayOrderRegression
    {
        private static void Check(bool condition, string message)
        {
            if (!condition) throw new System.Exception(message);
        }

        public static async System.Threading.Tasks.Task Run()
        {
            var order = new DisplayOrder();
            string visible = "";
            Check(await order.ApplyAsync(20, () => { visible = "new result"; return System.Threading.Tasks.Task.FromResult(true); }), "new show rejected");
            Check(!await order.ApplyAsync(10, () => { visible = ""; return System.Threading.Tasks.Task.FromResult(true); }), "late hide accepted");
            Check(visible == "new result", "late hide cleared the new result");
            Check(!order.Retire(10) && order.IsCurrent(20), "old disconnect retired new processing");
            Check(await order.ApplyAsync(30, () => { visible = ""; return System.Threading.Tasks.Task.FromResult(true); }), "new hide rejected");
            Check(!await order.ApplyAsync(20, () => { visible = "old result"; return System.Threading.Tasks.Task.FromResult(true); }), "late show accepted");
            Check(visible == "", "late show revived hidden content");
            Check(await order.ApplyAsync(30, () => System.Threading.Tasks.Task.FromResult(true)), "same-operation phase rejected");
            Check(order.Retire(30), "current disconnect rejected");
            Check(!await order.ApplyAsync(30, () => System.Threading.Tasks.Task.FromResult(true)), "closed operation revived");
            Check(!await order.ApplyAsync(0, () => System.Threading.Tasks.Task.FromResult(true)), "unsequenced message accepted");

            // Hold an old async mutation, queue a new result, then deliver the old hide.
            // No sleeps: the task completion sources determine the interleaving.
            order = new DisplayOrder();
            var release = new System.Threading.Tasks.TaskCompletionSource<bool>();
            var started = new System.Threading.Tasks.TaskCompletionSource<bool>();
            var old = order.ApplyAsync(1, async () => {
                started.SetResult(true);
                await release.Task;
                if (!order.IsCurrent(1)) return false;
                visible = "old result";
                return true;
            });
            await started.Task;
            var newer = order.ApplyAsync(2, () => { visible = "new result"; return System.Threading.Tasks.Task.FromResult(true); });
            Check(newer.IsCompleted, "new result waited for an old async call");
            Check(!await order.ApplyAsync(1, () => { visible = ""; return System.Threading.Tasks.Task.FromResult(true); }), "queued late hide accepted");
            release.SetResult(true);
            Check(!await old && await newer && visible == "new result", "old continuation beat the new result");

            // Retirement also invalidates a suspended continuation; a new operation can retry.
            release = new System.Threading.Tasks.TaskCompletionSource<bool>();
            bool applied = false;
            var closed = order.ApplyAsync(3, async () => {
                await release.Task;
                if (!order.IsCurrent(3)) return false;
                applied = true;
                return true;
            });
            Check(order.Retire(3), "current disconnect rejected");
            release.SetResult(true);
            Check(!await closed && !applied, "closed continuation mutated the view");
            Check(!await order.ApplyAsync(3, () => System.Threading.Tasks.Task.FromResult(true)), "closed queued action revived");
            Check(await order.ApplyAsync(4, () => System.Threading.Tasks.Task.FromResult(true)), "retry after disconnect rejected");
        }
    }
}
