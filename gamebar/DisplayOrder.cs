using System;
using System.Threading.Tasks;

namespace BlinkVerdict.GameBar
{
    // Accessed on the UI dispatcher. New operations never wait for an older
    // async call; continuations must confirm IsCurrent before mutating the view.
    sealed class DisplayOrder
    {
        private long newest;
        private bool retired;

        public bool IsCurrent(long operation) { return operation > 0 && operation == newest && !retired; }

        public bool Retire(long operation)
        {
            if (!IsCurrent(operation)) return false;
            retired = true;
            return true;
        }

        public Task<bool> ApplyAsync(long operation, Func<Task<bool>> apply)
        {
            if (operation <= 0 || operation < newest || (operation == newest && retired))
                return Task.FromResult(false);
            if (operation > newest) { newest = operation; retired = false; }
            return apply();
        }
    }
}
