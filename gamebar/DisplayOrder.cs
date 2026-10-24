using System;
using System.Threading;
using System.Threading.Tasks;

namespace BlinkVerdict.GameBar
{
    // Accessed on the view's UI dispatcher. Reserve the newest operation before
    // awaiting, then serialize UI mutations so an older continuation cannot win.
    sealed class DisplayOrder
    {
        private long newest;
        private bool retired;
        private readonly SemaphoreSlim applying = new SemaphoreSlim(1, 1);

        public bool IsCurrent(long operation) { return operation > 0 && operation == newest && !retired; }

        public bool Retire(long operation)
        {
            if (!IsCurrent(operation)) return false;
            retired = true;
            return true;
        }

        public async Task<bool> ApplyAsync(long operation, Func<Task<bool>> apply)
        {
            if (operation <= 0 || operation < newest || (operation == newest && retired)) return false;
            if (operation > newest) { newest = operation; retired = false; }
            await applying.WaitAsync();
            try
            {
                if (!IsCurrent(operation)) return false;
                return await apply();
            }
            finally { applying.Release(); }
        }
    }
}
